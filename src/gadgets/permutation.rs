use crate::field::{extension_field::Extendable, field_types::Field};
use crate::gates::switch::SwitchGate;
use crate::iop::generator::{GeneratedValues, SimpleGenerator};
use crate::iop::target::Target;
use crate::iop::witness::PartialWitness;
use crate::plonk::circuit_builder::CircuitBuilder;

impl<F: Extendable<D>, const D: usize> CircuitBuilder<F, D> {
    /// Assert that two lists of expressions evaluate to permutations of one another.
    pub fn assert_permutation<const CHUNK_SIZE: usize>(
        &mut self,
        a: Vec<[Target; CHUNK_SIZE]>,
        b: Vec<[Target; CHUNK_SIZE]>,
    ) {
        assert_eq!(
            a.len(),
            b.len(),
            "Permutation must have same number of inputs and outputs"
        );
        assert_eq!(a[0].len(), b[0].len(), "Chunk sizes must be the same");

        match a.len() {
            // Two empty lists are permutations of one another, trivially.
            0 => (),
            // Two singleton lists are permutations of one another as long as their items are equal.
            1 => {
                for e in 0..CHUNK_SIZE {
                    self.assert_equal(a[0][e], b[0][e])
                }
            }
            2 => self.assert_permutation_2x2(a[0], a[1], b[0], b[1]),
            // For larger lists, we recursively use two smaller permutation networks.
            //_ => self.assert_permutation_recursive(a, b)
            _ => self.assert_permutation_recursive(a, b),
        }
    }

    /// Assert that [a, b] is a permutation of [c, d].
    fn assert_permutation_2x2<const CHUNK_SIZE: usize>(
        &mut self,
        a: [Target; CHUNK_SIZE],
        b: [Target; CHUNK_SIZE],
        c: [Target; CHUNK_SIZE],
        d: [Target; CHUNK_SIZE],
    ) {
        let gate = SwitchGate::<F, D, CHUNK_SIZE>::new(1);
        let gate_index = self.add_gate(gate.clone(), vec![]);

        for e in 0..CHUNK_SIZE {
            self.route(a[e], Target::wire(gate_index, gate.wire_first_input(0, e)));
            self.route(b[e], Target::wire(gate_index, gate.wire_second_input(0, e)));
            self.route(c[e], Target::wire(gate_index, gate.wire_first_output(0, e)));
            self.route(
                d[e],
                Target::wire(gate_index, gate.wire_second_output(0, e)),
            );
        }
    }

    fn assert_permutation_recursive<const CHUNK_SIZE: usize>(
        &mut self,
        a: Vec<[Target; CHUNK_SIZE]>,
        b: Vec<[Target; CHUNK_SIZE]>,
    ) {
    }
}

struct PermutationGenerator<const CHUNK_SIZE: usize> {
    gate_index: usize,
}

impl<F: Field, const CHUNK_SIZE: usize> SimpleGenerator<F> for PermutationGenerator<CHUNK_SIZE> {
    fn dependencies(&self) -> Vec<Target> {
        todo!()
    }

    fn run_once(&self, witness: &PartialWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;
    use crate::field::crandall_field::CrandallField;
    use crate::field::extension_field::quartic::QuarticCrandallField;
    use crate::field::field_types::Field;
    use crate::iop::witness::PartialWitness;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::verifier::verify;

    fn test_permutation(size: usize) -> Result<()> {
        type F = CrandallField;
        type FF = QuarticCrandallField;
        let len = 1 << len_log;
        let config = CircuitConfig::large_config();
        let pw = PartialWitness::new(config.num_wires);
        let mut builder = CircuitBuilder::<F, 4>::new(config);
        let vec = FF::rand_vec(len);
        let v: Vec<_> = vec.iter().map(|x| builder.constant_extension(*x)).collect();

        for i in 0..len {
            let it = builder.constant(F::from_canonical_usize(i));
            let elem = builder.constant_extension(vec[i]);
            builder.random_access(it, elem, v.clone());
        }

        let data = builder.build();
        let proof = data.prove(pw)?;

        verify(proof, &data.verifier_only, &data.common)
    }
}
