use std::marker::PhantomData;

use itertools::izip;

use crate::field::extension_field::Extendable;
use crate::field::field_types::{Field, RichField};
use crate::gates::comparison::ComparisonGate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::{BoolTarget, Target};
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::util::ceil_div_usize;

pub struct MemoryOp<F: Field> {
    is_write: bool,
    address: F,
    timestamp: F,
    value: F,
}

#[derive(Clone, Debug)]
pub struct MemoryOpTarget {
    is_write: BoolTarget,
    address: Target,
    timestamp: Target,
    value: Target,
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn assert_permutation_memory_ops(&mut self, a: &[MemoryOpTarget], b: &[MemoryOpTarget]) {
        let a_chunks: Vec<Vec<Target>> = a
            .iter()
            .map(|op| vec![op.address, op.timestamp, op.is_write.target, op.value])
            .collect();
        let b_chunks: Vec<Vec<Target>> = b
            .iter()
            .map(|op| vec![op.address, op.timestamp, op.is_write.target, op.value])
            .collect();

        self.assert_permutation(a_chunks, b_chunks);
    }

    /// Add a ComparisonGate to assert that `lhs` is less than `rhs`, where their values are at most `bits` bits.
    pub fn assert_le(&mut self, lhs: Target, rhs: Target, bits: usize, num_chunks: usize) {
        let gate = ComparisonGate::new(bits, num_chunks);
        let gate_index = self.add_gate(gate.clone(), vec![]);

        self.connect(Target::wire(gate_index, gate.wire_first_input()), lhs);
        self.connect(Target::wire(gate_index, gate.wire_second_input()), rhs);
    }

    /// Sort memory operations by address value, then by timestamp value.
    /// This is done by combining address and timestamp into one field element (using their given bit lengths).
    pub fn sort_memory_ops(
        &mut self,
        ops: &[MemoryOpTarget],
        address_bits: usize,
        timestamp_bits: usize,
    ) -> Vec<MemoryOpTarget> {
        let n = ops.len();

        let combined_bits = address_bits + timestamp_bits;
        let chunk_bits = 3;
        let num_chunks = ceil_div_usize(combined_bits, chunk_bits);

        // This is safe because `assert_permutation` will force these targets (in the output list) to match the boolean values from the input list.
        let is_write_targets: Vec<_> = self
            .add_virtual_targets(n)
            .iter()
            .map(|&t| BoolTarget::new_unsafe(t))
            .collect();

        let address_targets = self.add_virtual_targets(n);
        let timestamp_targets = self.add_virtual_targets(n);
        let value_targets = self.add_virtual_targets(n);

        let output_targets: Vec<_> = izip!(
            is_write_targets,
            address_targets,
            timestamp_targets,
            value_targets
        )
        .map(|(i, a, t, v)| MemoryOpTarget {
            is_write: i,
            address: a,
            timestamp: t,
            value: v,
        })
        .collect();

        let two_n = self.constant(F::from_canonical_usize(1 << timestamp_bits));
        let address_timestamp_combined: Vec<_> = output_targets
            .iter()
            .map(|op| self.mul_add(op.address, two_n, op.timestamp))
            .collect();

        for i in 1..n {
            self.assert_le(
                address_timestamp_combined[i - 1],
                address_timestamp_combined[i],
                combined_bits,
                num_chunks,
            );
        }

        self.assert_permutation_memory_ops(ops, &output_targets);

        self.add_simple_generator(MemoryOpSortGenerator::<F, D> {
            input_ops: ops.to_vec(),
            output_ops: output_targets.clone(),
            _phantom: PhantomData,
        });

        output_targets
    }
}

#[derive(Debug)]
struct MemoryOpSortGenerator<F: RichField + Extendable<D>, const D: usize> {
    input_ops: Vec<MemoryOpTarget>,
    output_ops: Vec<MemoryOpTarget>,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F>
    for MemoryOpSortGenerator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        self.input_ops
            .iter()
            .map(|op| vec![op.is_write.target, op.address, op.timestamp, op.value])
            .flatten()
            .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let n = self.input_ops.len();
        debug_assert!(self.output_ops.len() == n);

        let mut ops: Vec<_> = self
            .input_ops
            .iter()
            .map(|op| {
                let is_write = witness.get_bool_target(op.is_write);
                let address = witness.get_target(op.address);
                let timestamp = witness.get_target(op.timestamp);
                let value = witness.get_target(op.value);
                MemoryOp {
                    is_write,
                    address,
                    timestamp,
                    value,
                }
            })
            .collect();

        ops.sort_unstable_by_key(|op| {
            (
                op.address.to_canonical_u64(),
                op.timestamp.to_canonical_u64(),
            )
        });

        for (op, out_op) in ops.iter().zip(&self.output_ops) {
            out_buffer.set_target(out_op.is_write.target, F::from_bool(op.is_write));
            out_buffer.set_target(out_op.address, op.address);
            out_buffer.set_target(out_op.timestamp, op.timestamp);
            out_buffer.set_target(out_op.value, op.value);
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use rand::{thread_rng, Rng};

    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::field::field_types::{Field, PrimeField};
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    fn test_sorting(size: usize, address_bits: usize, timestamp_bits: usize) -> Result<()> {
        type F = CrandallField;
        const D: usize = 4;

        let config = CircuitConfig::large_config();

        let mut pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let mut rng = thread_rng();
        let is_write_vals: Vec<_> = (0..size).map(|_| rng.gen_range(0..2) != 0).collect();
        let address_vals: Vec<_> = (0..size)
            .map(|_| F::from_canonical_u64(rng.gen_range(0..1 << address_bits as u64)))
            .collect();
        let timestamp_vals: Vec<_> = (0..size)
            .map(|_| F::from_canonical_u64(rng.gen_range(0..1 << timestamp_bits as u64)))
            .collect();
        let value_vals: Vec<_> = (0..size).map(|_| F::rand()).collect();

        let input_ops: Vec<MemoryOpTarget> = izip!(
            is_write_vals.clone(),
            address_vals.clone(),
            timestamp_vals.clone(),
            value_vals.clone()
        )
        .map(|(is_write, address, timestamp, value)| MemoryOpTarget {
            is_write: builder.constant_bool(is_write),
            address: builder.constant(address),
            timestamp: builder.constant(timestamp),
            value: builder.constant(value),
        })
        .collect();

        let combined_vals_u64: Vec<_> = timestamp_vals
            .iter()
            .zip(&address_vals)
            .map(|(&t, &a)| (a.to_canonical_u64() << timestamp_bits as u64) + t.to_canonical_u64())
            .collect();
        let mut input_ops_and_keys: Vec<_> =
            izip!(is_write_vals, address_vals, timestamp_vals, value_vals)
                .zip(combined_vals_u64)
                .collect::<Vec<_>>();
        input_ops_and_keys.sort_by_key(|(_, val)| val.clone());
        let input_ops_sorted: Vec<_> = input_ops_and_keys.iter().map(|(x, _)| x).collect();

        let output_ops =
            builder.sort_memory_ops(input_ops.as_slice(), address_bits, timestamp_bits);

        for i in 0..size {
            pw.set_bool_target(output_ops[i].is_write, input_ops_sorted[i].0);
            pw.set_target(output_ops[i].address, input_ops_sorted[i].1);
            pw.set_target(output_ops[i].timestamp, input_ops_sorted[i].2);
            pw.set_target(output_ops[i].value, input_ops_sorted[i].3);
        }

        let data = builder.build();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }

    #[test]
    fn test_sorting_small() -> Result<()> {
        let size = 5;
        let address_bits = 20;
        let timestamp_bits = 20;

        test_sorting(size, address_bits, timestamp_bits)
    }

    #[test]
    fn test_sorting_large() -> Result<()> {
        let size = 20;
        let address_bits = 20;
        let timestamp_bits = 20;

        test_sorting(size, address_bits, timestamp_bits)
    }
}
