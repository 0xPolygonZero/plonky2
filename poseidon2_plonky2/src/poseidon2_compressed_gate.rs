use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};
use core::marker::PhantomData;

use plonky2::field::extension::Extendable;
use plonky2::field::types::Field;
use plonky2::gates::gate::Gate;
use plonky2::gates::util::StridedConstraintConsumer;
use plonky2::hash::hash_types::RichField;
use crate::poseidon2_compressed_hash as poseidon2;
use crate::poseidon2_compressed_hash::Poseidon2c;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use plonky2::iop::target::Target;
use plonky2::iop::wire::Wire;
use plonky2::iop::witness::{PartitionWitness, Witness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};
use poseidon2::COMPRESSION_WIDTH;

/// Evaluates a full Poseidon2c permutation with 8 state elements.
///
/// This also has some extra features to make it suitable for efficiently verifying Merkle proofs.
/// It has a flag which can be used to swap the first four inputs with the next four, for ordering
/// sibling digests.
#[derive(Debug, Default)]
pub struct Poseidon2cGate<F: RichField + Extendable<D>, const D: usize>(PhantomData<F>);

implement_poseidon2_gate!(Poseidon2cGate, Poseidon2c, Poseidon2cGenerator, COMPRESSION_WIDTH);

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::types::Field;
    use plonky2::gates::gate_testing::{test_eval_fns,test_low_degree};
    use plonky2::iop::target::Target;
    use super::Poseidon2cGate;
    use crate::poseidon2_compressed_hash::{COMPRESSION_WIDTH, Poseidon2c};
    use plonky2::iop::wire::Wire;
    use plonky2::iop::witness::{PartialWitness, WitnessWrite};
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::GenericConfig;
    use crate::poseidon2_compressed_goldilock::Poseidon2cGoldilocksConfig;
    use crate::poseidon2_goldilock::Poseidon2GoldilocksConfig;

    #[test]
    fn wire_indices() {
        type F = GoldilocksField;
        type Gate = Poseidon2cGate<F, 4>;

        assert_eq!(Gate::wire_input(0), 0);
        assert_eq!(Gate::wire_input(7), 7);
        assert_eq!(Gate::wire_output(0), 8);
        assert_eq!(Gate::wire_output(7), 15);
        assert_eq!(Gate::WIRE_SWAP, 16);
        assert_eq!(Gate::wire_delta(0), 17);
        assert_eq!(Gate::wire_delta(3), 20);
        assert_eq!(Gate::wire_full_sbox_0(1, 0), 21);
        assert_eq!(Gate::wire_full_sbox_0(3, 0), 37);
        assert_eq!(Gate::wire_full_sbox_0(3, 7), 44);
        assert_eq!(Gate::wire_partial_sbox(0), 45);
        assert_eq!(Gate::wire_partial_sbox(21), 66);
        assert_eq!(Gate::wire_full_sbox_1(0, 0), 67);
        assert_eq!(Gate::wire_full_sbox_1(3, 0), 91);
        assert_eq!(Gate::wire_full_sbox_1(3, 7), 98);
    }

    #[test]
    fn generated_output() {
        const D: usize = 2;
        type C = Poseidon2cGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig {
            num_wires: 107,
            num_routed_wires: 80,
            ..CircuitConfig::standard_recursion_config()
        };
        let mut builder = CircuitBuilder::new(config);
        type Gate = Poseidon2cGate<F, D>;
        let gate = Gate::new();
        let row = builder.add_gate(gate, vec![]);
        for i in 0..COMPRESSION_WIDTH {
            builder.register_public_input(Target::wire(row, Gate::wire_output(i)));
        }
        let circuit = builder.build_prover::<C>();

        let permutation_inputs = (0..COMPRESSION_WIDTH)
            .map(F::from_canonical_usize)
            .collect::<Vec<_>>();

        let mut inputs = PartialWitness::new();
        inputs.set_wire(
            Wire {
                row,
                column: Gate::WIRE_SWAP,
            },
            F::ZERO,
        );
        for i in 0..COMPRESSION_WIDTH {
            inputs.set_wire(
                Wire {
                    row,
                    column: Gate::wire_input(i),
                },
                permutation_inputs[i],
            );
        }

        let proof = circuit.prove(inputs).unwrap();

        let expected_outputs: [F; COMPRESSION_WIDTH] =
            <F as Poseidon2c>::poseidon2(permutation_inputs.try_into().unwrap());
        expected_outputs.iter().zip(proof.public_inputs.iter())
            .for_each(|(expected_out, out)|
                assert_eq!(expected_out, out)
            );
    }

    #[test]
    fn low_degree() {
        type F = GoldilocksField;
        let gate = Poseidon2cGate::<F, 4>::new();
        test_low_degree(gate)
    }

    // This test requires the following modifications:
    // - Replace calls to "permute" with calls to "permute_c" in src/fri/prover.rs and src/iop/challenger.rs
    // - Replace "SPONGE_WIDTH" with "COMPRESSION_WIDTH" in src/iop/challenger.rs
    // use anyhow::Result;
    // use crate::gates::gate_testing::test_eval_fns;
    #[test]
    fn eval_fns() -> Result<()> {
         const D: usize = 2;
         type C = Poseidon2GoldilocksConfig;
         type F = <C as GenericConfig<D>>::F;
         let gate = Poseidon2cGate::<F, 2>::new();
         test_eval_fns::<F, C, _, D>(gate)
    }
}