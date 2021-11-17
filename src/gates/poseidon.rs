use std::marker::PhantomData;

use crate::field::extension_field::target::ExtensionTarget;
use crate::field::extension_field::Extendable;
use crate::field::field_types::{Field, RichField};
use crate::gates::gate::Gate;
use crate::gates::poseidon_mds::PoseidonMdsGate;
use crate::hash::poseidon;
use crate::hash::poseidon::Poseidon;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGenerator};
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::iop::witness::{PartitionWitness, Witness};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};

/// Evaluates a full Poseidon permutation with 12 state elements.
///
/// This also has some extra features to make it suitable for efficiently verifying Merkle proofs.
/// It has a flag which can be used to swap the first four inputs with the next four, for ordering
/// sibling digests.
#[derive(Debug)]
pub struct PoseidonGate<
    F: RichField + Extendable<D> + Poseidon<WIDTH>,
    const D: usize,
    const WIDTH: usize,
> where
    [(); WIDTH - 1]:,
{
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D> + Poseidon<WIDTH>, const D: usize, const WIDTH: usize>
    PoseidonGate<F, D, WIDTH>
where
    [(); WIDTH - 1]:,
{
    pub fn new() -> Self {
        PoseidonGate {
            _phantom: PhantomData,
        }
    }

    /// The wire index for the `i`th input to the permutation.
    pub fn wire_input(i: usize) -> usize {
        i
    }

    /// The wire index for the `i`th output to the permutation.
    pub fn wire_output(i: usize) -> usize {
        WIDTH + i
    }

    /// If this is set to 1, the first four inputs will be swapped with the next four inputs. This
    /// is useful for ordering hashes in Merkle proofs. Otherwise, this should be set to 0.
    pub const WIRE_SWAP: usize = 2 * WIDTH;

    const START_DELTA: usize = 2 * WIDTH + 1;

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
        debug_assert!(round < poseidon::HALF_N_FULL_ROUNDS);
        debug_assert!(i < WIDTH);
        Self::START_FULL_0 + WIDTH * (round - 1) + i
    }

    const START_PARTIAL: usize = Self::START_FULL_0 + WIDTH * (poseidon::HALF_N_FULL_ROUNDS - 1);

    /// A wire which stores the input of the S-box of the `round`-th round of the partial rounds.
    fn wire_partial_sbox(round: usize) -> usize {
        debug_assert!(round < poseidon::N_PARTIAL_ROUNDS);
        Self::START_PARTIAL + round
    }

    const START_FULL_1: usize = Self::START_PARTIAL + poseidon::N_PARTIAL_ROUNDS;

    /// A wire which stores the input of the `i`-th S-box of the `round`-th round of the second set
    /// of full rounds.
    fn wire_full_sbox_1(round: usize, i: usize) -> usize {
        debug_assert!(round < poseidon::HALF_N_FULL_ROUNDS);
        debug_assert!(i < WIDTH);
        Self::START_FULL_1 + WIDTH * round + i
    }

    /// End of wire indices, exclusive.
    fn end() -> usize {
        Self::START_FULL_1 + WIDTH * poseidon::HALF_N_FULL_ROUNDS
    }
}

impl<F: RichField + Extendable<D> + Poseidon<WIDTH>, const D: usize, const WIDTH: usize> Gate<F, D>
    for PoseidonGate<F, D, WIDTH>
where
    [(); WIDTH - 1]:,
{
    fn id(&self) -> String {
        format!("{:?}<WIDTH={}>", self, WIDTH)
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
        let mut state = [F::Extension::ZERO; WIDTH];
        for i in 0..4 {
            let delta_i = vars.local_wires[Self::wire_delta(i)];
            let input_lhs = Self::wire_input(i);
            let input_rhs = Self::wire_input(i + 4);
            state[i] = vars.local_wires[input_lhs] + delta_i;
            state[i + 4] = vars.local_wires[input_rhs] - delta_i;
        }
        for i in 8..WIDTH {
            state[i] = vars.local_wires[Self::wire_input(i)];
        }

        let mut round_ctr = 0;

        // First set of full rounds.
        for r in 0..poseidon::HALF_N_FULL_ROUNDS {
            <F as Poseidon<WIDTH>>::constant_layer_field(&mut state, round_ctr);
            if r != 0 {
                for i in 0..WIDTH {
                    let sbox_in = vars.local_wires[Self::wire_full_sbox_0(r, i)];
                    constraints.push(state[i] - sbox_in);
                    state[i] = sbox_in;
                }
            }
            <F as Poseidon<WIDTH>>::sbox_layer_field(&mut state);
            state = <F as Poseidon<WIDTH>>::mds_layer_field(&state);
            round_ctr += 1;
        }

        // Partial rounds.
        <F as Poseidon<WIDTH>>::partial_first_constant_layer(&mut state);
        state = <F as Poseidon<WIDTH>>::mds_partial_layer_init(&mut state);
        for r in 0..(poseidon::N_PARTIAL_ROUNDS - 1) {
            let sbox_in = vars.local_wires[Self::wire_partial_sbox(r)];
            constraints.push(state[0] - sbox_in);
            state[0] = <F as Poseidon<WIDTH>>::sbox_monomial(sbox_in);
            state[0] += F::Extension::from_canonical_u64(
                <F as Poseidon<WIDTH>>::FAST_PARTIAL_ROUND_CONSTANTS[r],
            );
            state = <F as Poseidon<WIDTH>>::mds_partial_layer_fast_field(&state, r);
        }
        let sbox_in = vars.local_wires[Self::wire_partial_sbox(poseidon::N_PARTIAL_ROUNDS - 1)];
        constraints.push(state[0] - sbox_in);
        state[0] = <F as Poseidon<WIDTH>>::sbox_monomial(sbox_in);
        state = <F as Poseidon<WIDTH>>::mds_partial_layer_fast_field(
            &state,
            poseidon::N_PARTIAL_ROUNDS - 1,
        );
        round_ctr += poseidon::N_PARTIAL_ROUNDS;

        // Second set of full rounds.
        for r in 0..poseidon::HALF_N_FULL_ROUNDS {
            <F as Poseidon<WIDTH>>::constant_layer_field(&mut state, round_ctr);
            for i in 0..WIDTH {
                let sbox_in = vars.local_wires[Self::wire_full_sbox_1(r, i)];
                constraints.push(state[i] - sbox_in);
                state[i] = sbox_in;
            }
            <F as Poseidon<WIDTH>>::sbox_layer_field(&mut state);
            state = <F as Poseidon<WIDTH>>::mds_layer_field(&state);
            round_ctr += 1;
        }

        for i in 0..WIDTH {
            constraints.push(state[i] - vars.local_wires[Self::wire_output(i)]);
        }

        constraints
    }

    fn eval_unfiltered_base(&self, vars: EvaluationVarsBase<F>) -> Vec<F> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        // Assert that `swap` is binary.
        let swap = vars.local_wires[Self::WIRE_SWAP];
        constraints.push(swap * swap.sub_one());

        // Assert that each delta wire is set properly: `delta_i = swap * (rhs - lhs)`.
        for i in 0..4 {
            let input_lhs = vars.local_wires[Self::wire_input(i)];
            let input_rhs = vars.local_wires[Self::wire_input(i + 4)];
            let delta_i = vars.local_wires[Self::wire_delta(i)];
            constraints.push(swap * (input_rhs - input_lhs) - delta_i);
        }

        // Compute the possibly-swapped input layer.
        let mut state = [F::ZERO; WIDTH];
        for i in 0..4 {
            let delta_i = vars.local_wires[Self::wire_delta(i)];
            let input_lhs = Self::wire_input(i);
            let input_rhs = Self::wire_input(i + 4);
            state[i] = vars.local_wires[input_lhs] + delta_i;
            state[i + 4] = vars.local_wires[input_rhs] - delta_i;
        }
        for i in 8..WIDTH {
            state[i] = vars.local_wires[Self::wire_input(i)];
        }

        let mut round_ctr = 0;

        // First set of full rounds.
        for r in 0..poseidon::HALF_N_FULL_ROUNDS {
            <F as Poseidon<WIDTH>>::constant_layer(&mut state, round_ctr);
            if r != 0 {
                for i in 0..WIDTH {
                    let sbox_in = vars.local_wires[Self::wire_full_sbox_0(r, i)];
                    constraints.push(state[i] - sbox_in);
                    state[i] = sbox_in;
                }
            }
            <F as Poseidon<WIDTH>>::sbox_layer(&mut state);
            state = <F as Poseidon<WIDTH>>::mds_layer(&state);
            round_ctr += 1;
        }

        // Partial rounds.
        <F as Poseidon<WIDTH>>::partial_first_constant_layer(&mut state);
        state = <F as Poseidon<WIDTH>>::mds_partial_layer_init(&mut state);
        for r in 0..(poseidon::N_PARTIAL_ROUNDS - 1) {
            let sbox_in = vars.local_wires[Self::wire_partial_sbox(r)];
            constraints.push(state[0] - sbox_in);
            state[0] = <F as Poseidon<WIDTH>>::sbox_monomial(sbox_in);
            state[0] +=
                F::from_canonical_u64(<F as Poseidon<WIDTH>>::FAST_PARTIAL_ROUND_CONSTANTS[r]);
            state = <F as Poseidon<WIDTH>>::mds_partial_layer_fast(&state, r);
        }
        let sbox_in = vars.local_wires[Self::wire_partial_sbox(poseidon::N_PARTIAL_ROUNDS - 1)];
        constraints.push(state[0] - sbox_in);
        state[0] = <F as Poseidon<WIDTH>>::sbox_monomial(sbox_in);
        state =
            <F as Poseidon<WIDTH>>::mds_partial_layer_fast(&state, poseidon::N_PARTIAL_ROUNDS - 1);
        round_ctr += poseidon::N_PARTIAL_ROUNDS;

        // Second set of full rounds.
        for r in 0..poseidon::HALF_N_FULL_ROUNDS {
            <F as Poseidon<WIDTH>>::constant_layer(&mut state, round_ctr);
            for i in 0..WIDTH {
                let sbox_in = vars.local_wires[Self::wire_full_sbox_1(r, i)];
                constraints.push(state[i] - sbox_in);
                state[i] = sbox_in;
            }
            <F as Poseidon<WIDTH>>::sbox_layer(&mut state);
            state = <F as Poseidon<WIDTH>>::mds_layer(&state);
            round_ctr += 1;
        }

        for i in 0..WIDTH {
            constraints.push(state[i] - vars.local_wires[Self::wire_output(i)]);
        }

        constraints
    }

    fn eval_unfiltered_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        // The naive method is more efficient if we have enough routed wires for PoseidonMdsGate.
        let use_mds_gate =
            builder.config.num_routed_wires >= PoseidonMdsGate::<F, D, WIDTH>::new().num_wires();

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
        let mut state = [builder.zero_extension(); WIDTH];
        for i in 0..4 {
            let delta_i = vars.local_wires[Self::wire_delta(i)];
            let input_lhs = vars.local_wires[Self::wire_input(i)];
            let input_rhs = vars.local_wires[Self::wire_input(i + 4)];
            state[i] = builder.add_extension(input_lhs, delta_i);
            state[i + 4] = builder.sub_extension(input_rhs, delta_i);
        }
        for i in 8..WIDTH {
            state[i] = vars.local_wires[Self::wire_input(i)];
        }

        let mut round_ctr = 0;

        // First set of full rounds.
        for r in 0..poseidon::HALF_N_FULL_ROUNDS {
            <F as Poseidon<WIDTH>>::constant_layer_recursive(builder, &mut state, round_ctr);
            if r != 0 {
                for i in 0..WIDTH {
                    let sbox_in = vars.local_wires[Self::wire_full_sbox_0(r, i)];
                    constraints.push(builder.sub_extension(state[i], sbox_in));
                    state[i] = sbox_in;
                }
            }
            <F as Poseidon<WIDTH>>::sbox_layer_recursive(builder, &mut state);
            state = <F as Poseidon<WIDTH>>::mds_layer_recursive(builder, &state);
            round_ctr += 1;
        }

        // Partial rounds.
        if use_mds_gate {
            for r in 0..poseidon::N_PARTIAL_ROUNDS {
                <F as Poseidon<WIDTH>>::constant_layer_recursive(builder, &mut state, round_ctr);
                let sbox_in = vars.local_wires[Self::wire_partial_sbox(r)];
                constraints.push(builder.sub_extension(state[0], sbox_in));
                state[0] = <F as Poseidon<WIDTH>>::sbox_monomial_recursive(builder, sbox_in);
                state = <F as Poseidon<WIDTH>>::mds_layer_recursive(builder, &state);
                round_ctr += 1;
            }
        } else {
            <F as Poseidon<WIDTH>>::partial_first_constant_layer_recursive(builder, &mut state);
            state = <F as Poseidon<WIDTH>>::mds_partial_layer_init_recursive(builder, &mut state);
            for r in 0..(poseidon::N_PARTIAL_ROUNDS - 1) {
                let sbox_in = vars.local_wires[Self::wire_partial_sbox(r)];
                constraints.push(builder.sub_extension(state[0], sbox_in));
                state[0] = <F as Poseidon<WIDTH>>::sbox_monomial_recursive(builder, sbox_in);
                let c = <F as Poseidon<WIDTH>>::FAST_PARTIAL_ROUND_CONSTANTS[r];
                let c = F::Extension::from_canonical_u64(c);
                let c = builder.constant_extension(c);
                state[0] = builder.add_extension(state[0], c);
                state =
                    <F as Poseidon<WIDTH>>::mds_partial_layer_fast_recursive(builder, &state, r);
            }
            let sbox_in = vars.local_wires[Self::wire_partial_sbox(poseidon::N_PARTIAL_ROUNDS - 1)];
            constraints.push(builder.sub_extension(state[0], sbox_in));
            state[0] = <F as Poseidon<WIDTH>>::sbox_monomial_recursive(builder, sbox_in);
            state = <F as Poseidon<WIDTH>>::mds_partial_layer_fast_recursive(
                builder,
                &state,
                poseidon::N_PARTIAL_ROUNDS - 1,
            );
            round_ctr += poseidon::N_PARTIAL_ROUNDS;
        }

        // Second set of full rounds.
        for r in 0..poseidon::HALF_N_FULL_ROUNDS {
            <F as Poseidon<WIDTH>>::constant_layer_recursive(builder, &mut state, round_ctr);
            for i in 0..WIDTH {
                let sbox_in = vars.local_wires[Self::wire_full_sbox_1(r, i)];
                constraints.push(builder.sub_extension(state[i], sbox_in));
                state[i] = sbox_in;
            }
            <F as Poseidon<WIDTH>>::sbox_layer_recursive(builder, &mut state);
            state = <F as Poseidon<WIDTH>>::mds_layer_recursive(builder, &state);
            round_ctr += 1;
        }

        for i in 0..WIDTH {
            constraints
                .push(builder.sub_extension(state[i], vars.local_wires[Self::wire_output(i)]));
        }

        constraints
    }

    fn generators(
        &self,
        gate_index: usize,
        _local_constants: &[F],
    ) -> Vec<Box<dyn WitnessGenerator<F>>> {
        let gen = PoseidonGenerator::<F, D, WIDTH> {
            gate_index,
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
        WIDTH * (poseidon::N_FULL_ROUNDS_TOTAL - 1) + poseidon::N_PARTIAL_ROUNDS + WIDTH + 1 + 4
    }
}

#[derive(Debug)]
struct PoseidonGenerator<
    F: RichField + Extendable<D> + Poseidon<WIDTH>,
    const D: usize,
    const WIDTH: usize,
> where
    [(); WIDTH - 1]:,
{
    gate_index: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D> + Poseidon<WIDTH>, const D: usize, const WIDTH: usize>
    SimpleGenerator<F> for PoseidonGenerator<F, D, WIDTH>
where
    [(); WIDTH - 1]:,
{
    fn dependencies(&self) -> Vec<Target> {
        (0..WIDTH)
            .map(|i| PoseidonGate::<F, D, WIDTH>::wire_input(i))
            .chain(Some(PoseidonGate::<F, D, WIDTH>::WIRE_SWAP))
            .map(|input| Target::wire(self.gate_index, input))
            .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) {
        let local_wire = |input| Wire {
            gate: self.gate_index,
            input,
        };

        let mut state = (0..WIDTH)
            .map(|i| witness.get_wire(local_wire(PoseidonGate::<F, D, WIDTH>::wire_input(i))))
            .collect::<Vec<_>>();

        let swap_value = witness.get_wire(local_wire(PoseidonGate::<F, D, WIDTH>::WIRE_SWAP));
        debug_assert!(swap_value == F::ZERO || swap_value == F::ONE);

        for i in 0..4 {
            let delta_i = swap_value * (state[i + 4] - state[i]);
            out_buffer.set_wire(
                local_wire(PoseidonGate::<F, D, WIDTH>::wire_delta(i)),
                delta_i,
            );
        }

        if swap_value == F::ONE {
            for i in 0..4 {
                state.swap(i, 4 + i);
            }
        }

        let mut state: [F; WIDTH] = state.try_into().unwrap();
        let mut round_ctr = 0;

        for r in 0..poseidon::HALF_N_FULL_ROUNDS {
            <F as Poseidon<WIDTH>>::constant_layer_field(&mut state, round_ctr);
            if r != 0 {
                for i in 0..WIDTH {
                    out_buffer.set_wire(
                        local_wire(PoseidonGate::<F, D, WIDTH>::wire_full_sbox_0(r, i)),
                        state[i],
                    );
                }
            }
            <F as Poseidon<WIDTH>>::sbox_layer_field(&mut state);
            state = <F as Poseidon<WIDTH>>::mds_layer_field(&state);
            round_ctr += 1;
        }

        <F as Poseidon<WIDTH>>::partial_first_constant_layer(&mut state);
        state = <F as Poseidon<WIDTH>>::mds_partial_layer_init(&mut state);
        for r in 0..(poseidon::N_PARTIAL_ROUNDS - 1) {
            out_buffer.set_wire(
                local_wire(PoseidonGate::<F, D, WIDTH>::wire_partial_sbox(r)),
                state[0],
            );
            state[0] = <F as Poseidon<WIDTH>>::sbox_monomial(state[0]);
            state[0] +=
                F::from_canonical_u64(<F as Poseidon<WIDTH>>::FAST_PARTIAL_ROUND_CONSTANTS[r]);
            state = <F as Poseidon<WIDTH>>::mds_partial_layer_fast_field(&state, r);
        }
        out_buffer.set_wire(
            local_wire(PoseidonGate::<F, D, WIDTH>::wire_partial_sbox(
                poseidon::N_PARTIAL_ROUNDS - 1,
            )),
            state[0],
        );
        state[0] = <F as Poseidon<WIDTH>>::sbox_monomial(state[0]);
        state = <F as Poseidon<WIDTH>>::mds_partial_layer_fast_field(
            &state,
            poseidon::N_PARTIAL_ROUNDS - 1,
        );
        round_ctr += poseidon::N_PARTIAL_ROUNDS;

        for r in 0..poseidon::HALF_N_FULL_ROUNDS {
            <F as Poseidon<WIDTH>>::constant_layer_field(&mut state, round_ctr);
            for i in 0..WIDTH {
                out_buffer.set_wire(
                    local_wire(PoseidonGate::<F, D, WIDTH>::wire_full_sbox_1(r, i)),
                    state[i],
                );
            }
            <F as Poseidon<WIDTH>>::sbox_layer_field(&mut state);
            state = <F as Poseidon<WIDTH>>::mds_layer_field(&state);
            round_ctr += 1;
        }

        for i in 0..WIDTH {
            out_buffer.set_wire(
                local_wire(PoseidonGate::<F, D, WIDTH>::wire_output(i)),
                state[i],
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::field::field_types::Field;
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::gates::poseidon::PoseidonGate;
    use crate::hash::hashing::SPONGE_WIDTH;
    use crate::hash::poseidon::Poseidon;
    use crate::iop::generator::generate_partial_witness;
    use crate::iop::wire::Wire;
    use crate::iop::witness::{PartialWitness, Witness};
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;

    #[test]
    fn wire_indices() {
        type F = GoldilocksField;
        const WIDTH: usize = 12;
        type Gate = PoseidonGate<F, 4, WIDTH>;

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
        type F = GoldilocksField;
        const WIDTH: usize = 12;

        let config = CircuitConfig {
            num_wires: 143,
            ..CircuitConfig::standard_recursion_config()
        };
        let mut builder = CircuitBuilder::new(config);
        type Gate = PoseidonGate<F, 4, WIDTH>;
        let gate = Gate::new();
        let gate_index = builder.add_gate(gate, vec![]);
        let circuit = builder.build_prover();

        let permutation_inputs = (0..WIDTH).map(F::from_canonical_usize).collect::<Vec<_>>();

        let mut inputs = PartialWitness::new();
        inputs.set_wire(
            Wire {
                gate: gate_index,
                input: Gate::WIRE_SWAP,
            },
            F::ZERO,
        );
        for i in 0..WIDTH {
            inputs.set_wire(
                Wire {
                    gate: gate_index,
                    input: Gate::wire_input(i),
                },
                permutation_inputs[i],
            );
        }

        let witness = generate_partial_witness(inputs, &circuit.prover_only, &circuit.common);

        let expected_outputs: [F; WIDTH] = F::poseidon(permutation_inputs.try_into().unwrap());
        for i in 0..WIDTH {
            let out = witness.get_wire(Wire {
                gate: 0,
                input: Gate::wire_output(i),
            });
            assert_eq!(out, expected_outputs[i]);
        }
    }

    #[test]
    fn low_degree() {
        type F = GoldilocksField;
        let gate = PoseidonGate::<F, 4, SPONGE_WIDTH>::new();
        test_low_degree(gate)
    }

    #[test]
    fn eval_fns() -> Result<()> {
        type F = GoldilocksField;
        let gate = PoseidonGate::<F, 4, SPONGE_WIDTH>::new();
        test_eval_fns(gate)
    }
}
