use std::marker::PhantomData;

use itertools::{izip, Itertools};

use crate::field::field_types::{PrimeField, RichField};
use crate::field::{extension_field::Extendable, field_types::Field};
use crate::gates::comparison::ComparisonGate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::{BoolTarget, Target};
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;

#[derive(Debug)]
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

    pub fn sort_memory_ops(
        &mut self,
        ops: &[MemoryOpTarget],
        address_bits: usize,
        timestamp_bits: usize,
    ) -> Vec<MemoryOpTarget> {
        let n = ops.len();

        let combined_bits = address_bits + timestamp_bits;
        let chunk_size = 3;

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
            .map(|op| self.mul_add(op.timestamp, two_n, op.address))
            .collect();

        for i in 1..n {
            let (gate, gate_index) = {
                let gate = ComparisonGate::new(combined_bits, chunk_size);
                let gate_index = self.add_gate(gate.clone(), vec![]);
                (gate, gate_index)
            };

            self.connect(
                Target::wire(gate_index, gate.wire_first_input()),
                address_timestamp_combined[i - 1],
            );
            self.connect(
                Target::wire(gate_index, gate.wire_second_input()),
                address_timestamp_combined[i],
            );
        }

        self.assert_permutation_memory_ops(ops, output_targets.as_slice());

        output_targets
    }
}

#[derive(Debug)]
struct MemoryOpSortGenerator<F: RichField> {
    input_ops: Vec<MemoryOpTarget>,
    output_ops: Vec<MemoryOpTarget>,
    address_bits: usize,
    timestamp_bits: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField> SimpleGenerator<F> for MemoryOpSortGenerator<F> {
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

        let combined_values_u64: Vec<_> = timestamp_values
            .iter()
            .zip(address_values.iter())
            .map(|(&t, &a)| {
                a.to_canonical_u64() * (1 << self.timestamp_bits as u64) + t.to_canonical_u64()
            })
            .collect();

        let mut input_ops_and_keys: Vec<_> = self
            .input_ops
            .iter()
            .zip(combined_values_u64)
            .collect::<Vec<_>>();
        input_ops_and_keys.sort_by(|(_, a_val), (_, b_val)| a_val.cmp(b_val));
        let input_ops_sorted: Vec<_> = input_ops_and_keys.iter().map(|(op, _)| op).collect();

        for i in 0..n {
            out_buffer.set_target(
                self.output_ops[i].is_write.target,
                witness.get_target(input_ops_sorted[i].is_write.target),
            );
            out_buffer.set_target(
                self.output_ops[i].address,
                witness.get_target(input_ops_sorted[i].address),
            );
            out_buffer.set_target(
                self.output_ops[i].timestamp,
                witness.get_target(input_ops_sorted[i].timestamp),
            );
            out_buffer.set_target(
                self.output_ops[i].value,
                witness.get_target(input_ops_sorted[i].value),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use rand::{seq::SliceRandom, thread_rng, Rng};

    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::field::field_types::Field;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    fn test_sorting(size: usize, address_bits: usize, timestamp_bits: usize) -> Result<()> {
        type F = CrandallField;
        const D: usize = 4;

        let config = CircuitConfig::large_zk_config();

        let pw = PartialWitness::new();
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

        let input_ops: Vec<MemoryOpTarget> =
            izip!(is_write_vals, address_vals, timestamp_vals, value_vals)
                .map(|(is_write, address, timestamp, value)| MemoryOpTarget {
                    is_write: builder.constant_bool(is_write),
                    address: builder.constant(address),
                    timestamp: builder.constant(timestamp),
                    value: builder.constant(value),
                })
                .collect();

        let _output_ops =
            builder.sort_memory_ops(input_ops.as_slice(), address_bits, timestamp_bits);

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
}
