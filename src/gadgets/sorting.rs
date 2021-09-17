use itertools::izip;
use std::marker::PhantomData;

use crate::field::field_types::RichField;
use crate::field::{extension_field::Extendable, field_types::Field};
use crate::gates::comparison::ComparisonGate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::{BoolTarget, Target};
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;

pub struct MemoryOpTarget {
    is_write: BoolTarget,
    address: Target,
    timestamp: Target,
    value: Target,
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    pub fn assert_permutation_memory_ops(&mut self, a: &[MemoryOpTarget], b: &[MemoryOpTarget]) {
        let a_chunks: Vec<Vec<Target>> = a.iter().map(|op| {
            vec![op.address, op.timestamp, op.is_write.target, op.value]
        }).collect();
        let b_chunks: Vec<Vec<Target>> = b.iter().map(|op| {
            vec![op.address, op.timestamp, op.is_write.target, op.value]
        }).collect();

        self.assert_permutation(a_chunks, b_chunks);
    }

    pub fn sort_memory_ops(&mut self, ops: &[MemoryOpTarget], address_bits: usize, timestamp_bits: usize) -> Vec<MemoryOpTarget> {
        let n = ops.len();

        let address_chunk_size = (address_bits as f64).sqrt() as usize;
        let timestamp_chunk_size = (timestamp_bits as f64).sqrt() as usize;

        let is_write_targets: Vec<_> = self.add_virtual_targets(n).iter().map(|&t| BoolTarget::new_unsafe(t)).collect();
        let address_targets = self.add_virtual_targets(n);
        let timestamp_targets = self.add_virtual_targets(n);
        let value_targets = self.add_virtual_targets(n);

        let output_targets: Vec<_> = izip!(is_write_targets, address_targets, timestamp_targets, value_targets).map(|(i, a, t, v)| {
            MemoryOpTarget {
                is_write: i,
                address: a,
                timestamp: t,
                value: v,
            }
        }).collect();

        for i in 1..n {
            let (address_gate, address_gate_index) = {
                let gate = ComparisonGate::new(address_bits, address_chunk_size);
                let gate_index = self.add_gate(gate.clone(), vec![]);
                (gate, gate_index)
            };

            self.connect(
                Target::wire(address_gate_index, address_gate.wire_first_input()),
                output_targets[i-1].address,
            );
            self.connect(
                Target::wire(address_gate_index, address_gate.wire_second_input()),
                output_targets[i].address,
            );

            let (timestamp_gate, timestamp_gate_index) = {
                let gate = ComparisonGate::new(timestamp_bits, timestamp_chunk_size);
                let gate_index = self.add_gate(gate.clone(), vec![]);
                (gate, gate_index)
            };

            self.connect(
                Target::wire(timestamp_gate_index, timestamp_gate.wire_first_input()),
                output_targets[i-1].timestamp,
            );
            self.connect(
                Target::wire(timestamp_gate_index, timestamp_gate.wire_second_input()),
                output_targets[i].timestamp,
            );
        }

        self.assert_permutation_memory_ops(ops, output_targets.as_slice());

        output_targets
    }
}

/*#[derive(Debug)]
struct MemoryOpSortGenerator<F: Field> {
    a: Vec<Vec<Target>>,
    b: Vec<Vec<Target>>,
    a_switches: Vec<Target>,
    b_switches: Vec<Target>,
    _phantom: PhantomData<F>,
}

impl<F: Field> SimpleGenerator<F> for MemoryOpSortGenerator<F> {
    fn dependencies(&self) -> Vec<Target> {
        self.a.iter().chain(&self.b).flatten().cloned().collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let a_values = self
            .a
            .iter()
            .map(|chunk| chunk.iter().map(|wire| witness.get_target(*wire)).collect())
            .collect();
        let b_values = self
            .b
            .iter()
            .map(|chunk| chunk.iter().map(|wire| witness.get_target(*wire)).collect())
            .collect();
        route(
            a_values,
            b_values,
            self.a_switches.clone(),
            self.b_switches.clone(),
            witness,
            out_buffer,
        );
    }
}*/

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use rand::{seq::SliceRandom, thread_rng};

    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::field::field_types::Field;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    fn test_permutation_good(size: usize) -> Result<()> {
        type F = CrandallField;
        const D: usize = 4;

        let config = CircuitConfig::large_zk_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let lst: Vec<F> = (0..size * 2).map(|n| F::from_canonical_usize(n)).collect();
        let a: Vec<Vec<Target>> = lst[..]
            .chunks(2)
            .map(|pair| vec![builder.constant(pair[0]), builder.constant(pair[1])])
            .collect();
        let mut b = a.clone();
        b.shuffle(&mut thread_rng());

        builder.assert_permutation(a, b);

        let data = builder.build();
        let proof = data.prove(pw).unwrap();

        verify(proof, &data.verifier_only, &data.common)
    }

    fn test_permutation_bad(size: usize) -> Result<()> {
        type F = CrandallField;
        const D: usize = 4;

        let config = CircuitConfig::large_zk_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let lst1: Vec<F> = F::rand_vec(size * 2);
        let lst2: Vec<F> = F::rand_vec(size * 2);
        let a: Vec<Vec<Target>> = lst1[..]
            .chunks(2)
            .map(|pair| vec![builder.constant(pair[0]), builder.constant(pair[1])])
            .collect();
        let b: Vec<Vec<Target>> = lst2[..]
            .chunks(2)
            .map(|pair| vec![builder.constant(pair[0]), builder.constant(pair[1])])
            .collect();

        builder.assert_permutation(a, b);

        let data = builder.build();
        data.prove(pw).unwrap();

        Ok(())
    }

    #[test]
    fn test_permutations_good() -> Result<()> {
        for n in 2..9 {
            test_permutation_good(n)?;
        }

        Ok(())
    }

    #[test]
    #[should_panic]
    fn test_permutation_bad_small() {
        let size = 2;

        test_permutation_bad(size).unwrap()
    }

    #[test]
    #[should_panic]
    fn test_permutation_bad_medium() {
        let size = 6;

        test_permutation_bad(size).unwrap()
    }

    #[test]
    #[should_panic]
    fn test_permutation_bad_large() {
        let size = 10;

        test_permutation_bad(size).unwrap()
    }
}
