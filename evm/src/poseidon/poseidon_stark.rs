use std::borrow::Borrow;
use std::iter::once;
use std::marker::PhantomData;

use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::hash::poseidon::Poseidon;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::timed;
use plonky2::util::timing::TimingTree;

use super::columns::{
    reg_cubed_full, reg_cubed_partial, reg_full_sbox_0, reg_full_sbox_1, reg_input_capacity,
    reg_output_capacity, reg_partial_sbox, PoseidonColumnsView, HALF_N_FULL_ROUNDS, NUM_COLUMNS,
    N_PARTIAL_ROUNDS, POSEIDON_COL_MAP, POSEIDON_DIGEST, POSEIDON_SPONGE_RATE,
    POSEIDON_SPONGE_WIDTH,
};
use crate::all_stark::Table;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cross_table_lookup::TableWithColumns;
use crate::evaluation_frame::{StarkEvaluationFrame, StarkFrame};
use crate::lookup::{Column, Filter};
use crate::stark::Stark;
use crate::util::trace_rows_to_poly_values;
use crate::witness::memory::MemoryAddress;

pub(crate) fn ctl_looked<F: Field>() -> TableWithColumns<F> {
    let mut columns = Column::singles(POSEIDON_COL_MAP.input).collect_vec();
    columns.extend(Column::singles(POSEIDON_COL_MAP.digest));
    TableWithColumns::new(
        *Table::Poseidon,
        columns,
        Some(Filter::new_simple(Column::single(
            POSEIDON_COL_MAP.not_padding,
        ))),
    )
}

#[derive(Copy, Clone, Debug)]
pub struct PoseidonOp<F: RichField>(pub [F; POSEIDON_SPONGE_WIDTH]);

#[derive(Copy, Clone, Default)]
pub struct PoseidonStark<F, const D: usize> {
    pub(crate) f: PhantomData<F>,
}

/// Information about a Poseidon operation needed for witness generation.
impl<F: RichField + Extendable<D>, const D: usize> PoseidonStark<F, D> {
    /// Generate the rows of the trace. Note that this does not generate the permuted columns used
    /// in our lookup arguments, as those are computed after transposing to column-wise form.
    fn generate_trace_rows(
        &self,
        operations: Vec<PoseidonOp<F>>,
        min_rows: usize,
    ) -> Vec<[F; NUM_COLUMNS]> {
        let num_rows = operations.len().max(min_rows).next_power_of_two();
        let mut rows = Vec::with_capacity(operations.len().max(min_rows));

        for op in operations {
            rows.push(self.generate_row_for_op(op));
        }

        // We generate "actual" rows for padding to avoid having to store
        // another power of x, on top of x^3 and x^6.
        let padding_row: [F; NUM_COLUMNS] = {
            let mut tmp_row = PoseidonColumnsView::default();
            let padding_inp = [F::ZERO; POSEIDON_SPONGE_WIDTH];
            Self::generate_perm(&mut tmp_row, padding_inp);
            tmp_row
        }
        .into();
        rows.resize(num_rows, padding_row);
        rows
    }

    fn generate_row_for_op(&self, op: PoseidonOp<F>) -> [F; NUM_COLUMNS] {
        let mut row = PoseidonColumnsView::default();
        Self::generate_perm(&mut row, op.0);
        row.not_padding = F::ONE;
        row.into()
    }

    fn generate_perm(row: &mut PoseidonColumnsView<F>, input: [F; POSEIDON_SPONGE_WIDTH]) {
        // Populate the round input for the first round.
        row.input.copy_from_slice(&input);

        let mut state = input;
        let mut round_ctr = 0;

        for r in 0..HALF_N_FULL_ROUNDS {
            <F as Poseidon>::constant_layer_field(&mut state, round_ctr);

            for i in 0..POSEIDON_SPONGE_WIDTH {
                // We do not need to store the first full_sbox_0 inputs, since they are
                // the permutation's inputs.
                if r != 0 {
                    row.full_sbox_0[reg_full_sbox_0(r, i)] = state[i];
                }
                // Generate x^3 and x^6 for the SBox layer constraints.
                row.cubed_full[reg_cubed_full(r, i)] = state[i].cube();

                // Apply x^7 to the state.
                state[i] *=
                    row.cubed_full[reg_cubed_full(r, i)] * row.cubed_full[reg_cubed_full(r, i)];
            }
            state = <F as Poseidon>::mds_layer_field(&state);
            round_ctr += 1;
        }

        <F as Poseidon>::partial_first_constant_layer(&mut state);
        state = <F as Poseidon>::mds_partial_layer_init(&state);
        for r in 0..(N_PARTIAL_ROUNDS - 1) {
            row.partial_sbox[reg_partial_sbox(r)] = state[0];

            // Generate x^3 for the SBox layer constraints.
            row.cubed_partial[reg_cubed_partial(r)] = state[0] * state[0] * state[0];

            state[0] *=
                row.cubed_partial[reg_cubed_partial(r)] * row.cubed_partial[reg_cubed_partial(r)];
            state[0] += F::from_canonical_u64(<F as Poseidon>::FAST_PARTIAL_ROUND_CONSTANTS[r]);
            state = <F as Poseidon>::mds_partial_layer_fast_field(&state, r);
        }

        row.partial_sbox[reg_partial_sbox(N_PARTIAL_ROUNDS - 1)] = state[0];
        // Generate x^3 and x^6 for the SBox layer constraints.
        row.cubed_partial[reg_cubed_partial(N_PARTIAL_ROUNDS - 1)] = state[0].cube();

        state[0] *= row.cubed_partial[reg_cubed_partial(N_PARTIAL_ROUNDS - 1)]
            * row.cubed_partial[reg_cubed_partial(N_PARTIAL_ROUNDS - 1)];
        state = <F as Poseidon>::mds_partial_layer_fast_field(&state, N_PARTIAL_ROUNDS - 1);
        round_ctr += N_PARTIAL_ROUNDS;

        for r in 0..HALF_N_FULL_ROUNDS {
            <F as Poseidon>::constant_layer_field(&mut state, round_ctr);
            for i in 0..POSEIDON_SPONGE_WIDTH {
                row.full_sbox_1[reg_full_sbox_1(r, i)] = state[i];
                // Generate x^3 and x^6 for the SBox layer constraints.
                row.cubed_full[reg_cubed_full(HALF_N_FULL_ROUNDS + r, i)] = state[i].cube();

                state[i] *= row.cubed_full[reg_cubed_full(HALF_N_FULL_ROUNDS + r, i)]
                    * row.cubed_full[reg_cubed_full(HALF_N_FULL_ROUNDS + r, i)];
            }
            state = <F as Poseidon>::mds_layer_field(&state);
            round_ctr += 1;
        }

        for i in 0..POSEIDON_DIGEST {
            let state_val = state[i].to_canonical_u64();
            let hi_limb = F::from_canonical_u32((state_val >> 32) as u32);
            row.pinv[i] =
                if let Some(inv) = (hi_limb - F::from_canonical_u32(u32::MAX)).try_inverse() {
                    inv
                } else {
                    F::ZERO
                };
            row.digest[2 * i] = F::from_canonical_u32(state_val as u32);
            row.digest[2 * i + 1] = hi_limb;
        }
        row.output_partial
            .copy_from_slice(&state[POSEIDON_DIGEST..POSEIDON_SPONGE_WIDTH]);
    }

    pub fn generate_trace(
        &self,
        operations: Vec<PoseidonOp<F>>,
        min_rows: usize,
        timing: &mut TimingTree,
    ) -> Vec<PolynomialValues<F>> {
        // Generate the witness, except for permuted columns in the lookup argument.
        let trace_rows = timed!(
            timing,
            "generate trace rows",
            self.generate_trace_rows(operations, min_rows)
        );
        let trace_polys = timed!(
            timing,
            "convert to PolynomialValues",
            trace_rows_to_poly_values(trace_rows)
        );
        trace_polys
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for PoseidonStark<F, D> {
    type EvaluationFrame<FE, P, const D2: usize> = StarkFrame<P, NUM_COLUMNS>
        where
            FE: FieldExtension<D2, BaseField = F>,
            P: PackedField<Scalar = FE>;

    type EvaluationFrameTarget = StarkFrame<ExtensionTarget<D>, NUM_COLUMNS>;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: &Self::EvaluationFrame<FE, P, D2>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        let lv: &[P; NUM_COLUMNS] = vars.get_local_values().try_into().unwrap();
        let lv: &PoseidonColumnsView<P> = lv.borrow();

        // Padding flag must be boolean.
        let not_padding = lv.not_padding;
        yield_constr.constraint(not_padding * (not_padding - P::ONES));

        // Compute the input layer.
        let mut state = lv.input;

        let mut round_ctr = 0;

        // First set of full rounds.
        for r in 0..HALF_N_FULL_ROUNDS {
            <F as Poseidon>::constant_layer_packed_field(&mut state, round_ctr);

            for i in 0..POSEIDON_SPONGE_WIDTH {
                if r != 0 {
                    let sbox_in = lv.full_sbox_0[reg_full_sbox_0(r, i)];
                    yield_constr.constraint(state[i] - sbox_in);
                    state[i] = sbox_in;
                }

                // Check that the powers were correctly generated.
                let cube = state[i] * state[i] * state[i];
                yield_constr.constraint(cube - lv.cubed_full[reg_cubed_full(r, i)]);

                state[i] *=
                    lv.cubed_full[reg_cubed_full(r, i)] * lv.cubed_full[reg_cubed_full(r, i)];
            }

            state = <F as Poseidon>::mds_layer_packed_field(&state);
            round_ctr += 1;
        }

        // Partial rounds.
        <F as Poseidon>::partial_first_constant_layer_packed(&mut state);
        state = <F as Poseidon>::mds_partial_layer_packed_init(&state);
        for r in 0..(N_PARTIAL_ROUNDS - 1) {
            let sbox_in = lv.partial_sbox[reg_partial_sbox(r)];
            yield_constr.constraint(state[0] - sbox_in);
            state[0] = sbox_in;

            // Check that the powers were generated correctly.
            let cube = state[0] * state[0] * state[0];
            yield_constr.constraint(cube - lv.cubed_partial[reg_cubed_partial(r)]);

            state[0] = lv.cubed_partial[reg_cubed_partial(r)]
                * lv.cubed_partial[reg_cubed_partial(r)]
                * sbox_in;
            state[0] +=
                P::Scalar::from_canonical_u64(<F as Poseidon>::FAST_PARTIAL_ROUND_CONSTANTS[r]);
            state = <F as Poseidon>::mds_partial_layer_fast_packed_field(&state, r);
        }
        let sbox_in = lv.partial_sbox[reg_partial_sbox(N_PARTIAL_ROUNDS - 1)];
        yield_constr.constraint(state[0] - sbox_in);
        state[0] = sbox_in;

        // Check that the powers were generated correctly.
        let cube = state[0] * state[0] * state[0];
        yield_constr.constraint(cube - lv.cubed_partial[reg_cubed_partial(N_PARTIAL_ROUNDS - 1)]);

        state[0] = lv.cubed_partial[reg_cubed_partial(N_PARTIAL_ROUNDS - 1)]
            * lv.cubed_partial[reg_cubed_partial(N_PARTIAL_ROUNDS - 1)]
            * sbox_in;
        state = <F as Poseidon>::mds_partial_layer_fast_packed_field(&state, N_PARTIAL_ROUNDS - 1);
        round_ctr += N_PARTIAL_ROUNDS;

        // Second set of full rounds.
        for r in 0..HALF_N_FULL_ROUNDS {
            <F as Poseidon>::constant_layer_packed_field(&mut state, round_ctr);
            for i in 0..POSEIDON_SPONGE_WIDTH {
                let sbox_in = lv.full_sbox_1[reg_full_sbox_1(r, i)];
                yield_constr.constraint(state[i] - sbox_in);
                state[i] = sbox_in;

                // Check that the powers were correctly generated.
                let cube = state[i] * state[i] * state[i];
                yield_constr
                    .constraint(cube - lv.cubed_full[reg_cubed_full(HALF_N_FULL_ROUNDS + r, i)]);

                state[i] *= lv.cubed_full[reg_cubed_full(HALF_N_FULL_ROUNDS + r, i)]
                    * lv.cubed_full[reg_cubed_full(HALF_N_FULL_ROUNDS + r, i)];
            }
            state = <F as Poseidon>::mds_layer_packed_field(&state);
            round_ctr += 1;
        }

        for i in 0..POSEIDON_DIGEST {
            yield_constr.constraint(
                state[i]
                    - (lv.digest[2 * i]
                        + lv.digest[2 * i + 1] * P::Scalar::from_canonical_u64(1 << 32)),
            );
        }
        for i in POSEIDON_DIGEST..POSEIDON_SPONGE_WIDTH {
            yield_constr.constraint(state[i] - lv.output_partial[i - POSEIDON_DIGEST])
        }

        // Ensure that the output limbs are written in canonical form.
        for i in 0..POSEIDON_DIGEST {
            let constr = ((lv.digest[2 * i + 1] - P::Scalar::from_canonical_u32(u32::MAX))
                * lv.pinv[i]
                - P::ONES)
                * lv.digest[2 * i];
            yield_constr.constraint(constr);
        }
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: &Self::EvaluationFrameTarget,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let lv: &[ExtensionTarget<D>; NUM_COLUMNS] = vars.get_local_values().try_into().unwrap();
        let lv: &PoseidonColumnsView<ExtensionTarget<D>> = lv.borrow();

        // Padding flag must be boolean.
        let not_padding = lv.not_padding;
        let constr = builder.mul_sub_extension(not_padding, not_padding, not_padding);
        yield_constr.constraint(builder, constr);

        // Compute the input layer.
        let mut state = lv.input;

        let mut round_ctr = 0;

        // First set of full rounds.
        for r in 0..HALF_N_FULL_ROUNDS {
            <F as Poseidon>::constant_layer_circuit(builder, &mut state, round_ctr);
            for i in 0..POSEIDON_SPONGE_WIDTH {
                if r != 0 {
                    let sbox_in = lv.full_sbox_0[reg_full_sbox_0(r, i)];
                    let constr = builder.sub_extension(state[i], sbox_in);
                    yield_constr.constraint(builder, constr);
                    state[i] = sbox_in;
                }

                // Check that the powers were correctly generated.
                let cube = builder.mul_many_extension([state[i], state[i], state[i]]);
                let constr = builder.sub_extension(cube, lv.cubed_full[reg_cubed_full(r, i)]);
                yield_constr.constraint(builder, constr);

                // Update the i'th element of the state.
                state[i] = builder.mul_many_extension([
                    state[i],
                    lv.cubed_full[reg_cubed_full(r, i)],
                    lv.cubed_full[reg_cubed_full(r, i)],
                ]);
            }

            state = <F as Poseidon>::mds_layer_circuit(builder, &state);
            round_ctr += 1;
        }

        // Partial rounds.
        <F as Poseidon>::partial_first_constant_layer_circuit(builder, &mut state);
        state = <F as Poseidon>::mds_partial_layer_init_circuit(builder, &state);
        for r in 0..(N_PARTIAL_ROUNDS - 1) {
            let sbox_in = lv.partial_sbox[reg_partial_sbox(r)];
            let constr = builder.sub_extension(state[0], sbox_in);
            yield_constr.constraint(builder, constr);
            state[0] = sbox_in;

            // Check that the powers were generated correctly.
            let cube = builder.mul_many_extension([state[0], state[0], state[0]]);
            let constr = builder.sub_extension(cube, lv.cubed_partial[reg_cubed_partial(r)]);
            yield_constr.constraint(builder, constr);

            // Update state[0].
            state[0] = builder.mul_many_extension([
                lv.cubed_partial[reg_cubed_partial(r)],
                lv.cubed_partial[reg_cubed_partial(r)],
                sbox_in,
            ]);
            state[0] = builder.add_const_extension(
                state[0],
                F::from_canonical_u64(<F as Poseidon>::FAST_PARTIAL_ROUND_CONSTANTS[r]),
            );
            state = <F as Poseidon>::mds_partial_layer_fast_circuit(builder, &state, r);
        }
        let sbox_in = lv.partial_sbox[reg_partial_sbox(N_PARTIAL_ROUNDS - 1)];
        let constr = builder.sub_extension(state[0], sbox_in);
        yield_constr.constraint(builder, constr);
        state[0] = sbox_in;

        // Check that the powers were generated correctly.
        let mut constr = builder.mul_many_extension([state[0], state[0], state[0]]);
        constr = builder.sub_extension(
            constr,
            lv.cubed_partial[reg_cubed_partial(N_PARTIAL_ROUNDS - 1)],
        );
        yield_constr.constraint(builder, constr);

        state[0] = builder.mul_many_extension([
            lv.cubed_partial[reg_cubed_partial(N_PARTIAL_ROUNDS - 1)],
            lv.cubed_partial[reg_cubed_partial(N_PARTIAL_ROUNDS - 1)],
            sbox_in,
        ]);
        state =
            <F as Poseidon>::mds_partial_layer_fast_circuit(builder, &state, N_PARTIAL_ROUNDS - 1);
        round_ctr += N_PARTIAL_ROUNDS;

        // Second set of full rounds.
        for r in 0..HALF_N_FULL_ROUNDS {
            <F as Poseidon>::constant_layer_circuit(builder, &mut state, round_ctr);
            for i in 0..POSEIDON_SPONGE_WIDTH {
                let sbox_in = lv.full_sbox_1[reg_full_sbox_1(r, i)];
                let constr = builder.sub_extension(state[i], sbox_in);
                yield_constr.constraint(builder, constr);
                state[i] = sbox_in;

                // Check that the powers were correctly generated.
                let mut constr = builder.mul_many_extension([state[i], state[i], state[i]]);
                constr = builder.sub_extension(
                    constr,
                    lv.cubed_full[reg_cubed_full(HALF_N_FULL_ROUNDS + r, i)],
                );
                yield_constr.constraint(builder, constr);

                // Update the i'th element of the state.
                state[i] = builder.mul_many_extension([
                    lv.cubed_full[reg_cubed_full(HALF_N_FULL_ROUNDS + r, i)],
                    lv.cubed_full[reg_cubed_full(HALF_N_FULL_ROUNDS + r, i)],
                    state[i],
                ]);
            }

            state = <F as Poseidon>::mds_layer_circuit(builder, &state);
            round_ctr += 1;
        }

        for i in 0..POSEIDON_DIGEST {
            let val = builder.mul_const_add_extension(
                F::from_canonical_u64(1 << 32),
                lv.digest[2 * i + 1],
                lv.digest[2 * i],
            );
            let constr = builder.sub_extension(state[i], val);
            yield_constr.constraint(builder, constr);
        }
        for i in POSEIDON_DIGEST..POSEIDON_SPONGE_WIDTH {
            let constr = builder.sub_extension(state[i], lv.output_partial[i - POSEIDON_DIGEST]);
            yield_constr.constraint(builder, constr);
        }

        // Ensure that the output limbs are written in canonical form.
        for i in 0..POSEIDON_DIGEST {
            let mut constr = builder.arithmetic_extension(
                F::ONE,
                F::NEG_ONE * F::from_canonical_u32(u32::MAX),
                lv.digest[2 * i + 1],
                lv.pinv[i],
                lv.pinv[i],
            );
            constr = builder.mul_sub_extension(lv.digest[2 * i], constr, lv.digest[2 * i]);

            yield_constr.constraint(builder, constr);
        }
    }

    fn constraint_degree(&self) -> usize {
        3
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Borrow;

    use anyhow::Result;
    use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
    use plonky2::field::polynomial::PolynomialValues;
    use plonky2::field::types::{Field, PrimeField64, Sample};
    use plonky2::fri::oracle::PolynomialBatch;
    use plonky2::hash::poseidon::Poseidon;
    use plonky2::iop::challenger::Challenger;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::timed;
    use plonky2::util::timing::TimingTree;

    use crate::config::StarkConfig;
    use crate::cross_table_lookup::{CtlData, CtlZData, GrandProductChallengeSet};
    use crate::lookup::GrandProductChallenge;
    // use crate::cross_table_lookup::{
    //     CtlData, CtlZData, GrandProductChallenge, GrandProductChallengeSet,
    // };
    use crate::memory::segments::Segment;
    use crate::poseidon::columns::{
        PoseidonColumnsView, POSEIDON_DIGEST, POSEIDON_SPONGE_RATE, POSEIDON_SPONGE_WIDTH,
    };
    use crate::poseidon::poseidon_stark::{PoseidonOp, PoseidonStark};
    use crate::prover::prove_single_table;
    use crate::stark_testing::{test_stark_circuit_constraints, test_stark_low_degree};
    use crate::witness::memory::MemoryAddress;

    #[test]
    fn test_stark_degree() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = PoseidonStark<F, D>;

        let stark = S {
            f: Default::default(),
        };
        test_stark_low_degree(stark)
    }

    #[test]
    fn test_stark_circuit() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = PoseidonStark<F, D>;

        let stark = S {
            f: Default::default(),
        };
        test_stark_circuit_constraints::<F, C, S, D>(stark)
    }

    #[test]
    fn poseidon_correctness_test() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = PoseidonStark<F, D>;

        let stark = S {
            f: Default::default(),
        };

        let input = PoseidonOp(F::rand_array());
        let rows = stark.generate_trace_rows(vec![input], 8);
        assert_eq!(rows.len(), 8);
        let row: PoseidonColumnsView<F> = rows[0].into();
        let expected = F::poseidon(input.0);
        assert_eq!(
            std::array::from_fn::<_, 4, _>(
                |i| row.digest[2 * i] + row.digest[2 * i + 1] * F::from_canonical_u64(1 << 32)
            ),
            expected[0..POSEIDON_DIGEST]
        );
        assert_eq!(
            row.output_partial,
            expected[POSEIDON_DIGEST..POSEIDON_SPONGE_WIDTH]
        );

        Ok(())
    }

    fn init_logger() {
        let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "debug"));
    }
}
