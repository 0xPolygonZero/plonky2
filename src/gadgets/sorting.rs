use itertools::izip;

use crate::field::extension_field::Extendable;
use crate::field::field_types::RichField;
use crate::gates::comparison::ComparisonGate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::{BoolTarget, Target};
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::util::ceil_div_usize;

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

    /// Add a ComparisonGate to
    /// Returns the gate and its index
    pub fn assert_le(
        &mut self,
        lhs: Target,
        rhs: Target,
        bits: usize,
        num_chunks: usize,
    ) -> (ComparisonGate<F, D>, usize) {
        let gate = ComparisonGate::new(bits, num_chunks);
        let gate_index = self.add_gate(gate.clone(), vec![]);

        self.connect(Target::wire(gate_index, gate.wire_first_input()), lhs);
        self.connect(Target::wire(gate_index, gate.wire_second_input()), rhs);

        (gate, gate_index)
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

        let mut gates = Vec::new();
        let mut gate_indices = Vec::new();
        for i in 1..n {
            let (gate, gate_index) = self.assert_le(
                address_timestamp_combined[i - 1],
                address_timestamp_combined[i],
                combined_bits,
                num_chunks,
            );

            gate_indices.push(gate_index);
            gates.push(gate);
        }

        self.assert_permutation_memory_ops(ops, output_targets.as_slice());

        self.add_simple_generator(MemoryOpSortGenerator::<F, D> {
            input_ops: ops.to_vec(),
            gate_indices,
            gates: gates.clone(),
            output_ops: output_targets.clone(),
            address_bits,
            timestamp_bits,
        });

        output_targets
    }
}

#[derive(Debug)]
struct MemoryOpSortGenerator<F: RichField + Extendable<D>, const D: usize> {
    input_ops: Vec<MemoryOpTarget>,
    gate_indices: Vec<usize>,
    gates: Vec<ComparisonGate<F, D>>,
    output_ops: Vec<MemoryOpTarget>,
    address_bits: usize,
    timestamp_bits: usize,
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

        let (timestamp_values, address_values): (Vec<_>, Vec<_>) = self
            .input_ops
            .iter()
            .map(|op| {
                (
                    witness.get_target(op.timestamp),
                    witness.get_target(op.address),
                )
            })
            .unzip();

        let combined_values: Vec<_> = timestamp_values
            .iter()
            .zip(&address_values)
            .map(|(&t, &a)| {
                F::from_canonical_u64(
                    (a.to_canonical_u64() << self.timestamp_bits as u64) + t.to_canonical_u64(),
                )
            })
            .collect();

        let mut input_ops_and_keys: Vec<_> = self
            .input_ops
            .iter()
            .zip(combined_values)
            .collect::<Vec<_>>();
        input_ops_and_keys.sort_by_key(|(_, val)| val.to_canonical_u64());

        for i in 0..n {
            out_buffer.set_target(
                self.output_ops[i].is_write.target,
                witness.get_target(input_ops_and_keys[i].0.is_write.target),
            );
            out_buffer.set_target(
                self.output_ops[i].address,
                witness.get_target(input_ops_and_keys[i].0.address),
            );
            out_buffer.set_target(
                self.output_ops[i].timestamp,
                witness.get_target(input_ops_and_keys[i].0.timestamp),
            );
            out_buffer.set_target(
                self.output_ops[i].value,
                witness.get_target(input_ops_and_keys[i].0.value),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use anyhow::Result;
    use rand::{seq::SliceRandom, thread_rng, Rng};

    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::field::field_types::{Field, PrimeField};
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    fn test_sorting(size: usize, address_bits: usize, timestamp_bits: usize) -> Result<()> {
        type F = CrandallField;
        const D: usize = 4;

        let config = CircuitConfig::large_zk_config();

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
