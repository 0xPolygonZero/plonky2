use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::hash::hashing::SPONGE_WIDTH;
use plonky2::hash::poseidon::{Poseidon, HALF_N_FULL_ROUNDS, N_PARTIAL_ROUNDS};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use starky::vars::{StarkEvaluationTargets, StarkEvaluationVars};

use crate::public_input_layout::NUM_PUBLIC_INPUTS;
use crate::registers::permutation::*;
use crate::registers::NUM_COLUMNS;

fn constant_layer<F, FE, P, const D: usize>(
    mut state: [P; SPONGE_WIDTH],
    round: usize,
) -> [P; SPONGE_WIDTH]
where
    F: Poseidon,
    FE: FieldExtension<D, BaseField = F>,
    P: PackedField<Scalar = FE>,
{
    // One day I might actually vectorize this, but today is not that day.
    for i in 0..P::WIDTH {
        let mut unpacked_state = [P::Scalar::default(); SPONGE_WIDTH];
        for j in 0..SPONGE_WIDTH {
            unpacked_state[j] = state[j].as_slice()[i];
        }
        F::constant_layer_field(&mut unpacked_state, round);
        for j in 0..SPONGE_WIDTH {
            state[j].as_slice_mut()[i] = unpacked_state[j];
        }
    }
    state
}

fn mds_layer<F, FE, P, const D: usize>(mut state: [P; SPONGE_WIDTH]) -> [P; SPONGE_WIDTH]
where
    F: Poseidon,
    FE: FieldExtension<D, BaseField = F>,
    P: PackedField<Scalar = FE>,
{
    for i in 0..P::WIDTH {
        let mut unpacked_state = [P::Scalar::default(); SPONGE_WIDTH];
        for j in 0..SPONGE_WIDTH {
            unpacked_state[j] = state[j].as_slice()[i];
        }
        unpacked_state = F::mds_layer_field(&unpacked_state);
        for j in 0..SPONGE_WIDTH {
            state[j].as_slice_mut()[i] = unpacked_state[j];
        }
    }
    state
}

pub(crate) fn generate_permutation_unit<F: Poseidon>(values: &mut [F; NUM_COLUMNS]) {
    // Load inputs.
    let mut state = [F::ZERO; SPONGE_WIDTH];
    for i in 0..SPONGE_WIDTH {
        state[i] = values[col_input(i)];
    }

    for r in 0..HALF_N_FULL_ROUNDS {
        F::constant_layer(&mut state, r);

        for i in 0..SPONGE_WIDTH {
            let state_cubed = state[i].cube();
            values[col_full_first_mid_sbox(r, i)] = state_cubed;
            state[i] *= state_cubed.square(); // Form state ** 7.
        }

        state = F::mds_layer(&state);

        for i in 0..SPONGE_WIDTH {
            values[col_full_first_after_mds(r, i)] = state[i];
        }
    }

    for r in 0..N_PARTIAL_ROUNDS {
        F::constant_layer(&mut state, HALF_N_FULL_ROUNDS + r);

        let state0_cubed = state[0].cube();
        values[col_partial_mid_sbox(r)] = state0_cubed;
        state[0] *= state0_cubed.square(); // Form state ** 7.
        values[col_partial_after_sbox(r)] = state[0];

        state = F::mds_layer(&state);
    }

    for r in 0..HALF_N_FULL_ROUNDS {
        F::constant_layer(&mut state, HALF_N_FULL_ROUNDS + N_PARTIAL_ROUNDS + r);

        for i in 0..SPONGE_WIDTH {
            let state_cubed = state[i].cube();
            values[col_full_second_mid_sbox(r, i)] = state_cubed;
            state[i] *= state_cubed.square(); // Form state ** 7.
        }

        state = F::mds_layer(&state);

        for i in 0..SPONGE_WIDTH {
            values[col_full_second_after_mds(r, i)] = state[i];
        }
    }
}

#[inline]
pub(crate) fn eval_permutation_unit<F, FE, P, const D: usize>(
    vars: StarkEvaluationVars<FE, P, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut ConstraintConsumer<P>,
) where
    F: Poseidon,
    FE: FieldExtension<D, BaseField = F>,
    P: PackedField<Scalar = FE>,
{
    let local_values = &vars.local_values;

    // Load inputs.
    let mut state = [P::ZEROS; SPONGE_WIDTH];
    for i in 0..SPONGE_WIDTH {
        state[i] = local_values[col_input(i)];
    }

    for r in 0..HALF_N_FULL_ROUNDS {
        state = constant_layer(state, r);

        for i in 0..SPONGE_WIDTH {
            let state_cubed = state[i] * state[i].square();
            yield_constr.constraint(state_cubed - local_values[col_full_first_mid_sbox(r, i)]);
            let state_cubed = local_values[col_full_first_mid_sbox(r, i)];
            state[i] *= state_cubed.square(); // Form state ** 7.
        }

        state = mds_layer(state);

        for i in 0..SPONGE_WIDTH {
            yield_constr.constraint(state[i] - local_values[col_full_first_after_mds(r, i)]);
            state[i] = local_values[col_full_first_after_mds(r, i)];
        }
    }

    for r in 0..N_PARTIAL_ROUNDS {
        state = constant_layer(state, HALF_N_FULL_ROUNDS + r);

        let state0_cubed = state[0] * state[0].square();
        yield_constr.constraint(state0_cubed - local_values[col_partial_mid_sbox(r)]);
        let state0_cubed = local_values[col_partial_mid_sbox(r)];
        state[0] *= state0_cubed.square(); // Form state ** 7.
        yield_constr.constraint(state[0] - local_values[col_partial_after_sbox(r)]);
        state[0] = local_values[col_partial_after_sbox(r)];

        state = mds_layer(state);
    }

    for r in 0..HALF_N_FULL_ROUNDS {
        state = constant_layer(state, HALF_N_FULL_ROUNDS + N_PARTIAL_ROUNDS + r);

        for i in 0..SPONGE_WIDTH {
            let state_cubed = state[i] * state[i].square();
            yield_constr.constraint(state_cubed - local_values[col_full_second_mid_sbox(r, i)]);
            let state_cubed = local_values[col_full_second_mid_sbox(r, i)];
            state[i] *= state_cubed.square(); // Form state ** 7.
        }

        state = mds_layer(state);

        for i in 0..SPONGE_WIDTH {
            yield_constr.constraint(state[i] - local_values[col_full_second_after_mds(r, i)]);
            state[i] = local_values[col_full_second_after_mds(r, i)];
        }
    }
}

pub(crate) fn eval_permutation_unit_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    vars: StarkEvaluationTargets<D, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let zero = builder.zero_extension();
    let local_values = &vars.local_values;

    // Load inputs.
    let mut state = [zero; SPONGE_WIDTH];
    for i in 0..SPONGE_WIDTH {
        state[i] = local_values[col_input(i)];
    }

    for r in 0..HALF_N_FULL_ROUNDS {
        F::constant_layer_circuit(builder, &mut state, r);

        for i in 0..SPONGE_WIDTH {
            let state_cubed = builder.cube_extension(state[i]);
            let diff =
                builder.sub_extension(state_cubed, local_values[col_full_first_mid_sbox(r, i)]);
            yield_constr.constraint(builder, diff);
            let state_cubed = local_values[col_full_first_mid_sbox(r, i)];
            state[i] = builder.mul_many_extension([state[i], state_cubed, state_cubed]);
            // Form state ** 7.
        }

        state = F::mds_layer_circuit(builder, &state);

        for i in 0..SPONGE_WIDTH {
            let diff =
                builder.sub_extension(state[i], local_values[col_full_first_after_mds(r, i)]);
            yield_constr.constraint(builder, diff);
            state[i] = local_values[col_full_first_after_mds(r, i)];
        }
    }

    for r in 0..N_PARTIAL_ROUNDS {
        F::constant_layer_circuit(builder, &mut state, HALF_N_FULL_ROUNDS + r);

        let state0_cubed = builder.cube_extension(state[0]);
        let diff = builder.sub_extension(state0_cubed, local_values[col_partial_mid_sbox(r)]);
        yield_constr.constraint(builder, diff);
        let state0_cubed = local_values[col_partial_mid_sbox(r)];
        state[0] = builder.mul_many_extension([state[0], state0_cubed, state0_cubed]); // Form state ** 7.
        let diff = builder.sub_extension(state[0], local_values[col_partial_after_sbox(r)]);
        yield_constr.constraint(builder, diff);
        state[0] = local_values[col_partial_after_sbox(r)];

        state = F::mds_layer_circuit(builder, &state);
    }

    for r in 0..HALF_N_FULL_ROUNDS {
        F::constant_layer_circuit(
            builder,
            &mut state,
            HALF_N_FULL_ROUNDS + N_PARTIAL_ROUNDS + r,
        );

        for i in 0..SPONGE_WIDTH {
            let state_cubed = builder.cube_extension(state[i]);
            let diff =
                builder.sub_extension(state_cubed, local_values[col_full_second_mid_sbox(r, i)]);
            yield_constr.constraint(builder, diff);
            let state_cubed = local_values[col_full_second_mid_sbox(r, i)];
            state[i] = builder.mul_many_extension([state[i], state_cubed, state_cubed]);
            // Form state ** 7.
        }

        state = F::mds_layer_circuit(builder, &state);

        for i in 0..SPONGE_WIDTH {
            let diff =
                builder.sub_extension(state[i], local_values[col_full_second_after_mds(r, i)]);
            yield_constr.constraint(builder, diff);
            state[i] = local_values[col_full_second_after_mds(r, i)];
        }
    }
}

#[cfg(test)]
mod tests {
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::types::{Field, Sample};
    use plonky2::hash::poseidon::Poseidon;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use starky::constraint_consumer::ConstraintConsumer;
    use starky::vars::StarkEvaluationVars;

    use crate::permutation_unit::{eval_permutation_unit, generate_permutation_unit, SPONGE_WIDTH};
    use crate::public_input_layout::NUM_PUBLIC_INPUTS;
    use crate::registers::permutation::{col_input, col_output};
    use crate::registers::NUM_COLUMNS;

    #[test]
    fn generate_eval_consistency() {
        type F = GoldilocksField;

        let mut values = [F::default(); NUM_COLUMNS];
        generate_permutation_unit(&mut values);

        let vars = StarkEvaluationVars {
            local_values: &values,
            next_values: &[F::default(); NUM_COLUMNS],
            public_inputs: &[F::default(); NUM_PUBLIC_INPUTS],
        };

        let mut constrant_consumer = ConstraintConsumer::new(
            vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
            GoldilocksField::ONE,
            GoldilocksField::ONE,
            GoldilocksField::ONE,
        );
        eval_permutation_unit(vars, &mut constrant_consumer);
        for &acc in &constrant_consumer.constraint_accs {
            assert_eq!(acc, GoldilocksField::ZERO);
        }
    }

    #[test]
    fn poseidon_result() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let state = [F::default(); SPONGE_WIDTH].map(|_| F::sample(&mut rng));

        // Get true Poseidon hash
        let target = GoldilocksField::poseidon(state);

        // Get result from `generate_permutation_unit`
        // Initialize `values` with randomness to test that the code doesn't rely on zero-filling.
        let mut values = [F::default(); NUM_COLUMNS].map(|_| F::sample(&mut rng));
        for i in 0..SPONGE_WIDTH {
            values[col_input(i)] = state[i];
        }
        generate_permutation_unit(&mut values);
        let mut result = [F::default(); SPONGE_WIDTH];
        for i in 0..SPONGE_WIDTH {
            result[i] = values[col_output(i)];
        }

        assert_eq!(target, result);
    }

    // TODO(JN): test degree
    // TODO(JN): test `eval_permutation_unit_recursively`
}
