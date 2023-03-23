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
use plonky2::hash::hashing::SPONGE_WIDTH;
use crate::poseidon2_hash as poseidon2;
use crate::poseidon2_hash::Poseidon2;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use plonky2::iop::target::Target;
use plonky2::iop::wire::Wire;
use plonky2::iop::witness::{PartitionWitness, Witness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// Evaluates a full Poseidon2 permutation with 12 state elements.
///
/// This also has some extra features to make it suitable for efficiently verifying Merkle proofs.
/// It has a flag which can be used to swap the first four inputs with the next four, for ordering
/// sibling digests.
#[derive(Debug, Default)]
pub struct Poseidon2Gate<F: RichField + Extendable<D>, const D: usize>(PhantomData<F>);

impl<F: RichField + Extendable<D>, const D: usize> Poseidon2Gate<F, D> {
    pub fn new() -> Self {
        Self(PhantomData)
    }

    /// The wire index for the `i`th input to the permutation.
    pub fn wire_input(i: usize) -> usize {
        i
    }

    /// The wire index for the `i`th output to the permutation.
    pub fn wire_output(i: usize) -> usize {
        SPONGE_WIDTH + i
    }

    /// If this is set to 1, the first four inputs will be swapped with the next four inputs. This
    /// is useful for ordering hashes in Merkle proofs. Otherwise, this should be set to 0.
    pub const WIRE_SWAP: usize = 2 * SPONGE_WIDTH;

    const START_DELTA: usize = 2 * SPONGE_WIDTH + 1;

    /// A wire which stores `swap * (input[i + 4] - input[i])`; used to compute the swapped inputs.
    fn wire_delta(i: usize) -> usize {
        assert!(i < 4);
        Self::START_DELTA + i
    }

    const START_FULL_0: usize = Self::START_DELTA + 4;

    /// A wire which stores the input of the `i`-th S-box of the `round`-th round of the first set
    /// of full rounds.
    fn wire_full_sbox_0(round: usize, i: usize) -> usize {
        debug_assert!(
            round != 0,
            "First round S-box inputs are not stored as wires"
        );
        debug_assert!(round < poseidon2::HALF_N_FULL_ROUNDS);
        debug_assert!(i < SPONGE_WIDTH);
        Self::START_FULL_0 + SPONGE_WIDTH * (round - 1) + i
    }

    const START_PARTIAL: usize =
        Self::START_FULL_0 + SPONGE_WIDTH * (poseidon2::HALF_N_FULL_ROUNDS - 1);

    /// A wire which stores the input of the S-box of the `round`-th round of the partial rounds.
    fn wire_partial_sbox(round: usize) -> usize {
        debug_assert!(round < poseidon2::N_PARTIAL_ROUNDS);
        Self::START_PARTIAL + round
    }

    const START_FULL_1: usize = Self::START_PARTIAL + poseidon2::N_PARTIAL_ROUNDS;

    /// A wire which stores the input of the `i`-th S-box of the `round`-th round of the second set
    /// of full rounds.
    fn wire_full_sbox_1(round: usize, i: usize) -> usize {
        debug_assert!(round < poseidon2::HALF_N_FULL_ROUNDS);
        debug_assert!(i < SPONGE_WIDTH);
        Self::START_FULL_1 + SPONGE_WIDTH * round + i
    }

    /// End of wire indices, exclusive.
    fn end() -> usize {
        Self::START_FULL_1 + SPONGE_WIDTH * poseidon2::HALF_N_FULL_ROUNDS
    }
}

impl<F: RichField + Extendable<D> + Poseidon2, const D: usize> Gate<F, D> for Poseidon2Gate<F, D> {
    fn id(&self) -> String {
        format!("{self:?}<WIDTH={SPONGE_WIDTH}>")
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        // Assert that `swap` is binary.
        let swap = vars.local_wires[Self::WIRE_SWAP];
        constraints.push(swap * (swap - F::Extension::ONE));

        // Assert that each delta wire is set properly: `delta_i = swap * (rhs - lhs)`.
        for i in 0..4 {
            let input_lhs = vars.local_wires[Self::wire_input(i)];
            let input_rhs = vars.local_wires[Self::wire_input(i + 4)];
            let delta_i = vars.local_wires[Self::wire_delta(i)];
            constraints.push(swap * (input_rhs - input_lhs) - delta_i);
        }

        // Compute the possibly-swapped input layer.
        let mut state = [F::Extension::ZERO; SPONGE_WIDTH];
        for i in 0..4 {
            let delta_i = vars.local_wires[Self::wire_delta(i)];
            let input_lhs = Self::wire_input(i);
            let input_rhs = Self::wire_input(i + 4);
            state[i] = vars.local_wires[input_lhs] + delta_i;
            state[i + 4] = vars.local_wires[input_rhs] - delta_i;
        }
        for i in 8..SPONGE_WIDTH {
            state[i] = vars.local_wires[Self::wire_input(i)];
        }

        let mut round_ctr = 0;

        // First external matrix
        <F as Poseidon2>::external_matrix_field(&mut state);

        // First set of full rounds.
        for r in 0..poseidon2::HALF_N_FULL_ROUNDS {
            <F as Poseidon2>::constant_layer_field(&mut state, round_ctr);
            if r != 0 {
                for i in 0..SPONGE_WIDTH {
                    let sbox_in = vars.local_wires[Self::wire_full_sbox_0(r, i)];
                    constraints.push(state[i] - sbox_in);
                    state[i] = sbox_in;
                }
            }
            <F as Poseidon2>::sbox_layer_field(&mut state);
            <F as Poseidon2>::external_matrix_field(&mut state);
            round_ctr += 1;
        }

        // Partial rounds.
        let mut constant_counter = poseidon2::HALF_N_FULL_ROUNDS * SPONGE_WIDTH;
        for r in 0..(poseidon2::N_PARTIAL_ROUNDS) {
            state[0] += F::Extension::from_canonical_u64(poseidon2::ALL_ROUND_CONSTANTS[constant_counter]);
            constant_counter += SPONGE_WIDTH;
            let sbox_in = vars.local_wires[Self::wire_partial_sbox(r)];
            constraints.push(state[0] - sbox_in);
            state[0] = <F as Poseidon2>::sbox_monomial(sbox_in);
            <F as Poseidon2>::internal_matrix_field(&mut state);
        }
        round_ctr += poseidon2::N_PARTIAL_ROUNDS;

        // Second set of full rounds.
        for r in 0..poseidon2::HALF_N_FULL_ROUNDS {
            <F as Poseidon2>::constant_layer_field(&mut state, round_ctr);
            for i in 0..SPONGE_WIDTH {
                let sbox_in = vars.local_wires[Self::wire_full_sbox_1(r, i)];
                constraints.push(state[i] - sbox_in);
                state[i] = sbox_in;
            }
            <F as Poseidon2>::sbox_layer_field(&mut state);
            <F as Poseidon2>::external_matrix_field(&mut state);
            round_ctr += 1;
        }

        for i in 0..SPONGE_WIDTH {
            constraints.push(state[i] - vars.local_wires[Self::wire_output(i)]);
        }

        constraints
    }

    fn eval_unfiltered_base_one(
        &self,
        vars: EvaluationVarsBase<F>,
        mut yield_constr: StridedConstraintConsumer<F>,
    ) {
        // Assert that `swap` is binary.
        let swap = vars.local_wires[Self::WIRE_SWAP];
        yield_constr.one(swap * swap.sub_one());

        // Assert that each delta wire is set properly: `delta_i = swap * (rhs - lhs)`.
        for i in 0..4 {
            let input_lhs = vars.local_wires[Self::wire_input(i)];
            let input_rhs = vars.local_wires[Self::wire_input(i + 4)];
            let delta_i = vars.local_wires[Self::wire_delta(i)];
            yield_constr.one(swap * (input_rhs - input_lhs) - delta_i);
        }

        // Compute the possibly-swapped input layer.
        let mut state = [F::ZERO; SPONGE_WIDTH];
        for i in 0..4 {
            let delta_i = vars.local_wires[Self::wire_delta(i)];
            let input_lhs = Self::wire_input(i);
            let input_rhs = Self::wire_input(i + 4);
            state[i] = vars.local_wires[input_lhs] + delta_i;
            state[i + 4] = vars.local_wires[input_rhs] - delta_i;
        }
        for i in 8..SPONGE_WIDTH {
            state[i] = vars.local_wires[Self::wire_input(i)];
        }

        let mut round_ctr = 0;

        // First external matrix
        <F as Poseidon2>::external_matrix(&mut state);

        // First set of full rounds.
        for r in 0..poseidon2::HALF_N_FULL_ROUNDS {
            <F as Poseidon2>::constant_layer(&mut state, round_ctr);
            if r != 0 {
                for i in 0..SPONGE_WIDTH {
                    let sbox_in = vars.local_wires[Self::wire_full_sbox_0(r, i)];
                    yield_constr.one(state[i] - sbox_in);
                    state[i] = sbox_in;
                }
            }
            <F as Poseidon2>::sbox_layer(&mut state);
            <F as Poseidon2>::external_matrix(&mut state);
            round_ctr += 1;
        }

        // Partial rounds.
        let mut constant_counter = poseidon2::HALF_N_FULL_ROUNDS * SPONGE_WIDTH;
        for r in 0..(poseidon2::N_PARTIAL_ROUNDS) {
            state[0] += F::from_canonical_u64(poseidon2::ALL_ROUND_CONSTANTS[constant_counter]);
            constant_counter += SPONGE_WIDTH;
            let sbox_in = vars.local_wires[Self::wire_partial_sbox(r)];
            yield_constr.one(state[0] - sbox_in);
            state[0] = <F as Poseidon2>::sbox_monomial(sbox_in);
            <F as Poseidon2>::internal_matrix(&mut state);
        }
        round_ctr += poseidon2::N_PARTIAL_ROUNDS;

        // Second set of full rounds.
        for r in 0..poseidon2::HALF_N_FULL_ROUNDS {
            <F as Poseidon2>::constant_layer(&mut state, round_ctr);
            for i in 0..SPONGE_WIDTH {
                let sbox_in = vars.local_wires[Self::wire_full_sbox_1(r, i)];
                yield_constr.one(state[i] - sbox_in);
                state[i] = sbox_in;
            }
            <F as Poseidon2>::sbox_layer(&mut state);
            <F as Poseidon2>::external_matrix(&mut state);
            round_ctr += 1;
        }

        for i in 0..SPONGE_WIDTH {
            yield_constr.one(state[i] - vars.local_wires[Self::wire_output(i)]);
        }
    }

    fn eval_unfiltered_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {

        let mut constraints = Vec::with_capacity(self.num_constraints());

        // Assert that `swap` is binary.
        let swap = vars.local_wires[Self::WIRE_SWAP];
        constraints.push(builder.mul_sub_extension(swap, swap, swap));

        // Assert that each delta wire is set properly: `delta_i = swap * (rhs - lhs)`.
        for i in 0..4 {
            let input_lhs = vars.local_wires[Self::wire_input(i)];
            let input_rhs = vars.local_wires[Self::wire_input(i + 4)];
            let delta_i = vars.local_wires[Self::wire_delta(i)];
            let diff = builder.sub_extension(input_rhs, input_lhs);
            constraints.push(builder.mul_sub_extension(swap, diff, delta_i));
        }

        // Compute the possibly-swapped input layer.
        let mut state = [builder.zero_extension(); SPONGE_WIDTH];
        for i in 0..4 {
            let delta_i = vars.local_wires[Self::wire_delta(i)];
            let input_lhs = vars.local_wires[Self::wire_input(i)];
            let input_rhs = vars.local_wires[Self::wire_input(i + 4)];
            state[i] = builder.add_extension(input_lhs, delta_i);
            state[i + 4] = builder.sub_extension(input_rhs, delta_i);
        }
        for i in 8..SPONGE_WIDTH {
            state[i] = vars.local_wires[Self::wire_input(i)];
        }

        let mut round_ctr = 0;

        // First external matrix
        <F as Poseidon2>::external_matrix_circuit(builder, &mut state);

        // First set of full rounds.
        for r in 0..poseidon2::HALF_N_FULL_ROUNDS {
            <F as Poseidon2>::constant_layer_circuit(builder, &mut state, round_ctr);
            if r != 0 {
                for i in 0..SPONGE_WIDTH {
                    let sbox_in = vars.local_wires[Self::wire_full_sbox_0(r, i)];
                    constraints.push(builder.sub_extension(state[i], sbox_in));
                    state[i] = sbox_in;
                }
            }
            <F as Poseidon2>::sbox_layer_circuit(builder, &mut state);
            <F as Poseidon2>::external_matrix_circuit(builder, &mut state);
            round_ctr += 1;
        }

        // Partial rounds.
        let mut constant_counter = poseidon2::HALF_N_FULL_ROUNDS * SPONGE_WIDTH;
        for r in 0..(poseidon2::N_PARTIAL_ROUNDS) {
            let c = poseidon2::ALL_ROUND_CONSTANTS[constant_counter];
            let c = F::Extension::from_canonical_u64(c);
            let c = builder.constant_extension(c);
            state[0] = builder.add_extension(state[0], c);
            constant_counter += SPONGE_WIDTH;
            let sbox_in = vars.local_wires[Self::wire_partial_sbox(r)];
            constraints.push(builder.sub_extension(state[0], sbox_in));
            state[0] = <F as Poseidon2>::sbox_monomial_circuit(builder, sbox_in);
            <F as Poseidon2>::internal_matrix_circuit(builder, &mut state);
        }
        round_ctr += poseidon2::N_PARTIAL_ROUNDS;

        // Second set of full rounds.
        for r in 0..poseidon2::HALF_N_FULL_ROUNDS {
            <F as Poseidon2>::constant_layer_circuit(builder, &mut state, round_ctr);
            for i in 0..SPONGE_WIDTH {
                let sbox_in = vars.local_wires[Self::wire_full_sbox_1(r, i)];
                constraints.push(builder.sub_extension(state[i], sbox_in));
                state[i] = sbox_in;
            }
            <F as Poseidon2>::sbox_layer_circuit(builder, &mut state);
            <F as Poseidon2>::external_matrix_circuit(builder, &mut state);
            round_ctr += 1;
        }

        for i in 0..SPONGE_WIDTH {
            constraints
                .push(builder.sub_extension(state[i], vars.local_wires[Self::wire_output(i)]));
        }

        constraints
    }

    fn generators(&self, row: usize, _local_constants: &[F]) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = Poseidon2Generator::<F, D> {
            row,
            _phantom: PhantomData,
        };
        vec![Box::new(gen.adapter())]
    }

    fn num_wires(&self) -> usize {
        Self::end()
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        7
    }

    fn num_constraints(&self) -> usize {
        SPONGE_WIDTH * (poseidon2::N_FULL_ROUNDS_TOTAL - 1)
            + poseidon2::N_PARTIAL_ROUNDS
            + SPONGE_WIDTH
            + 1
            + 4
    }
}

#[derive(Debug)]
struct Poseidon2Generator<F: RichField + Extendable<D> + Poseidon2, const D: usize> {
    row: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D> + Poseidon2, const D: usize> SimpleGenerator<F>
for Poseidon2Generator<F, D>
{
    fn dependencies(&self) -> Vec<Target> {
        (0..SPONGE_WIDTH)
            .map(|i| Poseidon2Gate::<F, D>::wire_input(i))
            .chain(Some(Poseidon2Gate::<F, D>::WIRE_SWAP))
            .map(|column| Target::wire(self.row, column))
            .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |column| Wire {
            row: self.row,
            column,
        };

        let mut state = (0..SPONGE_WIDTH)
            .map(|i| witness.get_wire(local_wire(Poseidon2Gate::<F, D>::wire_input(i))))
            .collect::<Vec<_>>();

        let swap_value = witness.get_wire(local_wire(Poseidon2Gate::<F, D>::WIRE_SWAP));
        debug_assert!(swap_value == F::ZERO || swap_value == F::ONE);

        for i in 0..4 {
            let delta_i = swap_value * (state[i + 4] - state[i]);
            out_buffer.set_wire(local_wire(Poseidon2Gate::<F, D>::wire_delta(i)), delta_i);
        }

        if swap_value == F::ONE {
            for i in 0..4 {
                state.swap(i, 4 + i);
            }
        }

        let mut state: [F; SPONGE_WIDTH] = state.try_into().unwrap();
        let mut round_ctr = 0;

        // First external matrix
        <F as Poseidon2>::external_matrix_field(&mut state);

        for r in 0..poseidon2::HALF_N_FULL_ROUNDS {
            <F as Poseidon2>::constant_layer_field(&mut state, round_ctr);
            if r != 0 {
                for i in 0..SPONGE_WIDTH {
                    out_buffer.set_wire(
                        local_wire(Poseidon2Gate::<F, D>::wire_full_sbox_0(r, i)),
                        state[i],
                    );
                }
            }
            <F as Poseidon2>::sbox_layer_field(&mut state);
            <F as Poseidon2>::external_matrix_field(&mut state);
            round_ctr += 1;
        }

        let mut constant_counter = poseidon2::HALF_N_FULL_ROUNDS * SPONGE_WIDTH;
        for r in 0..(poseidon2::N_PARTIAL_ROUNDS) {
            state[0] += F::from_canonical_u64(poseidon2::ALL_ROUND_CONSTANTS[constant_counter]);
            constant_counter += SPONGE_WIDTH;
            out_buffer.set_wire(
                local_wire(Poseidon2Gate::<F, D>::wire_partial_sbox(r)),
                state[0],
            );
            state[0] = <F as Poseidon2>::sbox_monomial(state[0]);
            <F as Poseidon2>::internal_matrix_field(&mut state);
        }
        round_ctr += poseidon2::N_PARTIAL_ROUNDS;

        for r in 0..poseidon2::HALF_N_FULL_ROUNDS {
            <F as Poseidon2>::constant_layer_field(&mut state, round_ctr);
            for i in 0..SPONGE_WIDTH {
                out_buffer.set_wire(
                    local_wire(Poseidon2Gate::<F, D>::wire_full_sbox_1(r, i)),
                    state[i],
                );
            }
            <F as Poseidon2>::sbox_layer_field(&mut state);
            <F as Poseidon2>::external_matrix_field(&mut state);
            round_ctr += 1;
        }

        for i in 0..SPONGE_WIDTH {
            out_buffer.set_wire(local_wire(Poseidon2Gate::<F, D>::wire_output(i)), state[i]);
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::types::Field;
    use plonky2::gates::gate_testing::{test_eval_fns, test_low_degree};
    use super::Poseidon2Gate;
    use plonky2::hash::hashing::SPONGE_WIDTH;
    use crate::poseidon2_hash::Poseidon2;
    use plonky2::iop::target::Target;
    use plonky2::iop::wire::Wire;
    use plonky2::iop::witness::{PartialWitness, WitnessWrite};
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::GenericConfig;
    use crate::poseidon2_goldilock::Poseidon2GoldilocksConfig;

    #[test]
    fn wire_indices() {
        type F = GoldilocksField;
        type Gate = Poseidon2Gate<F, 4>;

        assert_eq!(Gate::wire_input(0), 0);
        assert_eq!(Gate::wire_input(11), 11);
        assert_eq!(Gate::wire_output(0), 12);
        assert_eq!(Gate::wire_output(11), 23);
        assert_eq!(Gate::WIRE_SWAP, 24);
        assert_eq!(Gate::wire_delta(0), 25);
        assert_eq!(Gate::wire_delta(3), 28);
        assert_eq!(Gate::wire_full_sbox_0(1, 0), 29);
        assert_eq!(Gate::wire_full_sbox_0(3, 0), 53);
        assert_eq!(Gate::wire_full_sbox_0(3, 11), 64);
        assert_eq!(Gate::wire_partial_sbox(0), 65);
        assert_eq!(Gate::wire_partial_sbox(21), 86);
        assert_eq!(Gate::wire_full_sbox_1(0, 0), 87);
        assert_eq!(Gate::wire_full_sbox_1(3, 0), 123);
        assert_eq!(Gate::wire_full_sbox_1(3, 11), 134);
    }

    #[test]
    fn generated_output() {
        const D: usize = 2;
        type C = Poseidon2GoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let config = CircuitConfig {
            num_wires: 143,
            ..CircuitConfig::standard_recursion_config()
        };
        let mut builder = CircuitBuilder::new(config);
        type Gate = Poseidon2Gate<F, D>;
        let gate = Gate::new();
        let row = builder.add_gate(gate, vec![]);
        for i in 0..SPONGE_WIDTH {
            builder.register_public_input(Target::wire(row, Gate::wire_output(i)));
        }
        let circuit = builder.build_prover::<C>();

        let permutation_inputs = (0..SPONGE_WIDTH)
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
        for i in 0..SPONGE_WIDTH {
            inputs.set_wire(
                Wire {
                    row,
                    column: Gate::wire_input(i),
                },
                permutation_inputs[i],
            );
        }

        let proof = circuit.prove(inputs).unwrap();

        let expected_outputs: [F; SPONGE_WIDTH] =
            F::poseidon2(permutation_inputs.try_into().unwrap());
        expected_outputs.iter().zip(proof.public_inputs.iter())
            .for_each(|(expected_out, out)|
                assert_eq!(expected_out, out)
            );
    }

    #[test]
    fn low_degree() {
        type F = GoldilocksField;
        let gate = Poseidon2Gate::<F, 4>::new();
        test_low_degree(gate)
    }

    #[test]
    fn eval_fns() -> Result<()> {
        const D: usize = 2;
        type C = Poseidon2GoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let gate = Poseidon2Gate::<F, 2>::new();
        test_eval_fns::<F, C, _, D>(gate)
    }
}
