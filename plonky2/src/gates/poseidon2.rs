use std::marker::PhantomData;

use plonky2_field::extension::Extendable;
use plonky2_field::types::Field;

use crate::gates::gate::Gate;
use crate::gates::util::StridedConstraintConsumer;
use crate::hash::hash_types::RichField;
use crate::hash::poseidon2;
use crate::hash::poseidon2::{Poseidon2, ROUND_F_BEGIN, ROUND_F_END, ROUND_P, WIDTH};
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::generator::{GeneratedValues, SimpleGenerator, WitnessGeneratorRef};
use crate::iop::target::Target;
use crate::iop::wire::Wire;
use crate::iop::witness::{PartitionWitness, Witness, WitnessWrite};
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::CommonCircuitData;
use crate::plonk::vars::{EvaluationTargets, EvaluationVars, EvaluationVarsBase};
use crate::util::serialization::{Buffer, IoResult, Read, Write};

/// Evaluates a full Poseidon2 permutation with 12 state elements.
///
/// This also has some extra features to make it suitable for efficiently
/// verifying Merkle proofs. It has a flag which can be used to swap the first
/// four inputs with the next four, for ordering sibling digests.
#[derive(Debug)]
pub struct Poseidon2Gate<F: RichField + Extendable<D>, const D: usize> {
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> Poseidon2Gate<F, D> {
    pub fn new() -> Self {
        Poseidon2Gate {
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

    /// If this is set to 1, the first four inputs will be swapped with the next
    /// four inputs. This is useful for ordering hashes in Merkle proofs.
    /// Otherwise, this should be set to 0.
    pub const WIRE_SWAP: usize = 2 * WIDTH;

    const START_DELTA: usize = 2 * WIDTH + 1;

    /// A wire which stores `swap * (input[i + 4] - input[i])`; used to compute
    /// the swapped inputs.
    fn wire_delta(i: usize) -> usize {
        assert!(i < 4);
        Self::START_DELTA + i
    }

    const START_ROUND_F_BEGIN: usize = Self::START_DELTA + 4;

    /// A wire which stores the input of the `i`-th S-box of the `round`-th
    /// round of the first set of full rounds.
    fn wire_full_round_begin(round: usize, i: usize) -> usize {
        debug_assert!(
            round != 0,
            "First round S-box inputs are not stored as wires"
        );
        debug_assert!(round < poseidon2::ROUND_F_BEGIN);
        debug_assert!(i < WIDTH);
        Self::START_ROUND_F_BEGIN + WIDTH * (round - 1) + i
    }

    const START_PARTIAL: usize = Self::START_ROUND_F_BEGIN + WIDTH * (poseidon2::ROUND_F_BEGIN - 1);

    /// A wire which stores the input of the S-box of the `round`-th round of
    /// the partial rounds.
    fn wire_partial_round(round: usize) -> usize {
        debug_assert!(round < poseidon2::ROUND_P);
        Self::START_PARTIAL + round
    }

    const START_ROUND_F_END: usize = Self::START_PARTIAL + poseidon2::ROUND_P;

    /// A wire which stores the input of the `i`-th S-box of the `round`-th
    /// round of the second set of full rounds.
    fn wire_full_round_end(round: usize, i: usize) -> usize {
        debug_assert!(round < poseidon2::ROUND_F_BEGIN);
        debug_assert!(i < WIDTH);
        Self::START_ROUND_F_END + WIDTH * round + i
    }

    /// End of wire indices, exclusive.
    fn end() -> usize {
        Self::START_ROUND_F_END + WIDTH * poseidon2::ROUND_F_BEGIN
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for Poseidon2Gate<F, D> {
    fn id(&self) -> String {
        format!("{:?}<WIDTH={}>", self, WIDTH)
    }

    fn serialize(
        &self,
        _dst: &mut Vec<u8>,
        _common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<()> {
        Ok(())
    }

    fn deserialize(_src: &mut Buffer, _common_data: &CommonCircuitData<F, D>) -> IoResult<Self> {
        Ok(Poseidon2Gate::new())
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        let mut constraints = Vec::with_capacity(self.num_constraints());

        // Assert that `swap` is binary.
        let swap = vars.local_wires[Self::WIRE_SWAP];
        // 1 constraint
        constraints.push(swap * (swap - F::Extension::ONE));

        // Assert that each delta wire is set properly: `delta_i = swap * (rhs - lhs)`.
        // 4 constraint
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

        // M_E * X
        <F as Poseidon2>::matmul_external_field(&mut state);

        // External_i, i in {0 - R_F/2 -1}
        for r in 0..poseidon2::ROUND_F_BEGIN {
            <F as Poseidon2>::constant_layer_field(&mut state, r);
            //12 * 3 = 36 constraints
            if r != 0 {
                for i in 0..WIDTH {
                    let sbox_in = vars.local_wires[Self::wire_full_round_begin(r, i)];
                    constraints.push(state[i] - sbox_in);
                    state[i] = sbox_in;
                }
            }
            <F as Poseidon2>::sbox_layer_field(&mut state);
            <F as Poseidon2>::matmul_external_field(&mut state);
        }

        // Internal_i
        for r in 0..poseidon2::ROUND_P {
            state[0] += F::Extension::from_canonical_u64(<F as Poseidon2>::RC12_MID[r]);

            //22 constraints
            let sbox_in = vars.local_wires[Self::wire_partial_round(r)];
            constraints.push(state[0] - sbox_in);
            //state[0] = sbox_in;
            state[0] = <F as Poseidon2>::sbox_monomial(sbox_in);
            <F as Poseidon2>::matmul_internal_field(&mut state, &<F as Poseidon2>::MAT_DIAG12_M_1);
        }

        // External_i, i in {R_F/2 = R/F - 1}.
        for r in poseidon2::ROUND_F_BEGIN..poseidon2::ROUND_F_END {
            <F as Poseidon2>::constant_layer_field(&mut state, r);

            //12 * 4 = 48 constraints
            for i in 0..WIDTH {
                let sbox_in = vars.local_wires[Self::wire_full_round_end(r - ROUND_F_BEGIN, i)];
                constraints.push(state[i] - sbox_in);
                state[i] = sbox_in;
            }

            <F as Poseidon2>::sbox_layer_field(&mut state);
            <F as Poseidon2>::matmul_external_field(&mut state);
        }

        //12 constraints
        for i in 0..WIDTH {
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

        // M_E * X
        <F as Poseidon2>::matmul_external(&mut state);

        // External_i, i in {0 - R_F/2 -1}
        for r in 0..ROUND_F_BEGIN {
            <F as Poseidon2>::constant_layer(&mut state, r);
            if r != 0 {
                for i in 0..WIDTH {
                    let sbox_in = vars.local_wires[Self::wire_full_round_begin(r, i)];
                    yield_constr.one(state[i] - sbox_in);
                    state[i] = sbox_in;
                }
            }
            <F as Poseidon2>::sbox_layer(&mut state);
            <F as Poseidon2>::matmul_external(&mut state);
        }

        // Internal_i
        for r in 0..ROUND_P {
            // t_0 = x_0 + c_0^i
            state[0] += F::from_canonical_u64(<F as Poseidon2>::RC12_MID[r]);
            let sbox_in = vars.local_wires[Self::wire_partial_round(r)];
            yield_constr.one(state[0] - sbox_in);
            // t_1 = t_0^7
            state[0] = sbox_in;
            state[0] = <F as Poseidon2>::sbox_monomial(state[0]);
            // M_I * t_1
            <F as Poseidon2>::matmul_internal(&mut state, &<F as Poseidon2>::MAT_DIAG12_M_1);
        }

        // External_i, i in {R_F/2 = R/F - 1}
        for r in ROUND_F_BEGIN..ROUND_F_END {
            <F as Poseidon2>::constant_layer(&mut state, r);

            for i in 0..WIDTH {
                let sbox_in = vars.local_wires[Self::wire_full_round_end(r - ROUND_F_BEGIN, i)];
                yield_constr.one(state[i] - sbox_in);
                state[i] = sbox_in;
            }

            <F as Poseidon2>::sbox_layer(&mut state);
            <F as Poseidon2>::matmul_external(&mut state);
        }

        for i in 0..WIDTH {
            yield_constr.one(state[i] - vars.local_wires[Self::wire_output(i)]);
        }
    }

    fn eval_unfiltered_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        // The naive method is more efficient if we have enough routed wires for

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

        // M_E * X
        state = <F as Poseidon2>::matmul_external_circuit(builder, &mut state);

        // External_i, i in {0 - R_F/2 -1}
        for r in 0..poseidon2::ROUND_F_BEGIN {
            <F as Poseidon2>::constant_layer_circuit(builder, &mut state, r);
            if r != 0 {
                for i in 0..WIDTH {
                    let sbox_in = vars.local_wires[Self::wire_full_round_begin(r, i)];
                    constraints.push(builder.sub_extension(state[i], sbox_in));
                    state[i] = sbox_in;
                }
            }
            <F as Poseidon2>::sbox_layer_circuit(builder, &mut state);
            state = <F as Poseidon2>::matmul_external_circuit(builder, &mut state);
        }

        // Internal_i
        for r in 0..poseidon2::ROUND_P {
            let round_constant = F::Extension::from_canonical_u64(<F as Poseidon2>::RC12_MID[r]);
            let round_constant = builder.constant_extension(round_constant);
            state[0] = builder.add_extension(state[0], round_constant);

            let sbox_in = vars.local_wires[Self::wire_partial_round(r)];
            constraints.push(builder.sub_extension(state[0], sbox_in));
            //state[0] = sbox_in;
            state[0] = <F as Poseidon2>::sbox_monomial_circuit(builder, sbox_in);
            <F as Poseidon2>::matmul_internal_circuit(builder, &mut state);
        }

        // External_i, i in {R_F/2 = R/F - 1}.
        for r in poseidon2::ROUND_F_BEGIN..poseidon2::ROUND_F_END {
            <F as Poseidon2>::constant_layer_circuit(builder, &mut state, r);

            for i in 0..WIDTH {
                let sbox_in = vars.local_wires[Self::wire_full_round_end(r - ROUND_F_BEGIN, i)];
                constraints.push(builder.sub_extension(state[i], sbox_in));
                state[i] = sbox_in;
            }

            <F as Poseidon2>::sbox_layer_circuit(builder, &mut state);
            state = <F as Poseidon2>::matmul_external_circuit(builder, &mut state);
        }

        for i in 0..WIDTH {
            constraints
                .push(builder.sub_extension(state[i], vars.local_wires[Self::wire_output(i)]));
        }

        constraints
    }

    fn generators(&self, row: usize, _local_constants: &[F]) -> Vec<WitnessGeneratorRef<F, D>> {
        let gen = Poseidon2Generator::<F, D> {
            row,
            _phantom: PhantomData,
        };
        vec![WitnessGeneratorRef::new(gen.adapter())]
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
        WIDTH * (poseidon2::ROUND_F_END - 1) + poseidon2::ROUND_P + WIDTH + 1 + 4
    }
}

#[derive(Debug)]
struct Poseidon2Generator<F: RichField + Extendable<D> + Poseidon2, const D: usize> {
    row: usize,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F, D>
    for Poseidon2Generator<F, D>
{
    fn id(&self) -> String {
        "Poseidon2Generator".to_string()
    }

    fn serialize(&self, dst: &mut Vec<u8>, _common_data: &CommonCircuitData<F, D>) -> IoResult<()> {
        dst.write_usize(self.row)
    }

    fn deserialize(src: &mut Buffer, _common_data: &CommonCircuitData<F, D>) -> IoResult<Self> {
        let row = src.read_usize()?;
        Ok(Self {
            row,
            _phantom: PhantomData,
        })
    }

    fn dependencies(&self) -> Vec<Target> {
        (0..WIDTH)
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

        let mut state = (0..WIDTH)
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

        let mut state: [F; WIDTH] = state.try_into().unwrap();

        // M_E * X
        <F as Poseidon2>::matmul_external_field(&mut state);

        // External_i, i in {0 - R_F/2 -1}
        for r in 0..poseidon2::ROUND_F_BEGIN {
            <F as Poseidon2>::constant_layer_field(&mut state, r);
            if r != 0 {
                for i in 0..WIDTH {
                    out_buffer.set_wire(
                        local_wire(Poseidon2Gate::<F, D>::wire_full_round_begin(r, i)),
                        state[i],
                    );
                }
            }
            <F as Poseidon2>::sbox_layer_field(&mut state);
            <F as Poseidon2>::matmul_external_field(&mut state);
        }

        // Internal_i
        for r in 0..poseidon2::ROUND_P {
            state[0] += F::from_canonical_u64(<F as Poseidon2>::RC12_MID[r]);
            out_buffer.set_wire(
                local_wire(Poseidon2Gate::<F, D>::wire_partial_round(r)),
                state[0],
            );
            state[0] = <F as Poseidon2>::sbox_monomial(state[0]);
            <F as Poseidon2>::matmul_internal_field(&mut state, &<F as Poseidon2>::MAT_DIAG12_M_1);
        }

        // External_i, i in {R_F/2 = R/F - 1}.
        for r in poseidon2::ROUND_F_BEGIN..poseidon2::ROUND_F_END {
            <F as Poseidon2>::constant_layer_field(&mut state, r);

            for i in 0..WIDTH {
                out_buffer.set_wire(
                    local_wire(Poseidon2Gate::<F, D>::wire_full_round_end(
                        r - ROUND_F_BEGIN,
                        i,
                    )),
                    state[i],
                );
            }

            <F as Poseidon2>::sbox_layer_field(&mut state);
            <F as Poseidon2>::matmul_external_field(&mut state);
        }

        for i in 0..WIDTH {
            out_buffer.set_wire(local_wire(Poseidon2Gate::<F, D>::wire_output(i)), state[i]);
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2_field::goldilocks_field::GoldilocksField;
    use plonky2_field::types::Field;

    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::gates::poseidon2::Poseidon2Gate;
    use crate::hash::poseidon2::{Poseidon2, WIDTH};
    use crate::iop::generator::generate_partial_witness;
    use crate::iop::wire::Wire;
    use crate::iop::witness::{PartialWitness, Witness, WitnessWrite};
    use crate::plonk::circuit_builder::CircuitBuilder;
    use crate::plonk::circuit_data::CircuitConfig;
    use crate::plonk::config::{GenericConfig, Poseidon2GoldilocksConfig};

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
        let circuit = builder.build_prover::<C>();

        let permutation_inputs = (0..WIDTH).map(F::from_canonical_usize).collect::<Vec<_>>();

        let mut inputs = PartialWitness::new();
        inputs.set_wire(
            Wire {
                row,
                column: Gate::WIRE_SWAP,
            },
            F::ZERO,
        );
        for i in 0..WIDTH {
            inputs.set_wire(
                Wire {
                    row,
                    column: Gate::wire_input(i),
                },
                permutation_inputs[i],
            );
        }

        let witness = generate_partial_witness(inputs, &circuit.prover_only, &circuit.common);

        let expected_outputs: [F; WIDTH] = F::poseidon2(permutation_inputs.try_into().unwrap());
        for i in 0..WIDTH {
            let out = witness.get_wire(Wire {
                row: 0,
                column: Gate::wire_output(i),
            });
            assert_eq!(out, expected_outputs[i]);
        }
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
