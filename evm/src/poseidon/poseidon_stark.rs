use std::marker::PhantomData;

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
    col_input_limb, col_output_limb, full_sbox_0, full_sbox_1, partial_sbox, reg_cubed_full,
    reg_cubed_partial, reg_input_limb, reg_output_limb, reg_power_6_full, reg_power_6_partial,
    FILTER, HALF_N_FULL_ROUNDS, NUM_COLUMNS, N_PARTIAL_ROUNDS, POSEIDON_SPONGE_WIDTH,
};
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cross_table_lookup::Column;
use crate::evaluation_frame::{StarkEvaluationFrame, StarkFrame};
use crate::stark::Stark;
use crate::util::trace_rows_to_poly_values;

pub fn ctl_data<F: Field>() -> Vec<Column<F>> {
    let mut res: Vec<_> = (0..POSEIDON_SPONGE_WIDTH).map(col_input_limb).collect();
    res.extend((0..POSEIDON_SPONGE_WIDTH).map(col_output_limb));
    res
}

pub fn ctl_filter<F: Field>() -> Column<F> {
    Column::single(FILTER)
}
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
        inputs: Vec<[u64; POSEIDON_SPONGE_WIDTH]>,
        min_rows: usize,
    ) -> Vec<[F; NUM_COLUMNS]> {
        let num_rows = (inputs.len()).max(min_rows).next_power_of_two();

        let mut rows = Vec::with_capacity(num_rows);
        for input in inputs.iter() {
            let inps_for_perm = input
                .iter()
                .map(|&elt| F::from_canonical_u64(elt))
                .collect::<Vec<F>>()
                .try_into()
                .unwrap();
            let row_for_perm = self.generate_trace_row_for_perm(inps_for_perm);
            rows.push(row_for_perm);
        }
        while rows.len() < num_rows {
            // We generate "actual" rows for padding to avoid having to store
            // another power of x, on top of x^3 and x^6.
            let input = [F::ZERO; POSEIDON_SPONGE_WIDTH];
            let mut row = self.generate_trace_row_for_perm(input);
            row[FILTER] = F::ZERO;
            rows.push(row)
        }
        rows
    }

    // One row per permutation.
    fn generate_trace_row_for_perm(&self, input: [F; POSEIDON_SPONGE_WIDTH]) -> [F; NUM_COLUMNS] {
        let mut row = [F::ZERO; NUM_COLUMNS];
        row[FILTER] = F::ONE;

        // Populate the round input for the first round.
        for i in 0..POSEIDON_SPONGE_WIDTH {
            row[reg_input_limb(i)] = input[i];
        }

        let mut state: [F; POSEIDON_SPONGE_WIDTH] = input;
        let mut round_ctr = 0;

        for r in 0..HALF_N_FULL_ROUNDS {
            <F as Poseidon>::constant_layer_field(&mut state, round_ctr);

            for i in 0..POSEIDON_SPONGE_WIDTH {
                // We do not need to store the first full_sbox_0 inputs, since they are
                // the permutation's inputs.
                if r != 0 {
                    row[full_sbox_0(r, i)] = state[i];
                }
                // Generate x^3 and x^6 for the SBox layer constraints.
                row[reg_cubed_full(r, i)] = state[i] * state[i] * state[i];
                row[reg_power_6_full(r, i)] = row[reg_cubed_full(r, i)] * row[reg_cubed_full(r, i)];

                // Apply x^7 to the state.
                state[i] *= row[reg_power_6_full(r, i)];
            }
            state = <F as Poseidon>::mds_layer_field(&state);
            round_ctr += 1;
        }

        <F as Poseidon>::partial_first_constant_layer(&mut state);
        state = <F as Poseidon>::mds_partial_layer_init(&state);
        for r in 0..(N_PARTIAL_ROUNDS - 1) {
            row[partial_sbox(r)] = state[0];

            // Generate x^3 and x^6 for the SBox layer constraints.
            row[reg_cubed_partial(r)] = state[0] * state[0] * state[0];
            row[reg_power_6_partial(r)] = row[reg_cubed_partial(r)] * row[reg_cubed_partial(r)];
            state[0] *= row[reg_power_6_partial(r)];
            state[0] += F::from_canonical_u64(<F as Poseidon>::FAST_PARTIAL_ROUND_CONSTANTS[r]);
            state = <F as Poseidon>::mds_partial_layer_fast_field(&state, r);
        }

        row[partial_sbox(N_PARTIAL_ROUNDS - 1)] = state[0];
        // Generate x^3 and x^6 for the SBox layer constraints.
        row[reg_cubed_partial(N_PARTIAL_ROUNDS - 1)] = state[0] * state[0] * state[0];
        row[reg_power_6_partial(N_PARTIAL_ROUNDS - 1)] = row
            [reg_cubed_partial(N_PARTIAL_ROUNDS - 1)]
            * row[reg_cubed_partial(N_PARTIAL_ROUNDS - 1)];
        state[0] *= row[reg_power_6_partial(N_PARTIAL_ROUNDS - 1)];
        state = <F as Poseidon>::mds_partial_layer_fast_field(&state, N_PARTIAL_ROUNDS - 1);
        round_ctr += N_PARTIAL_ROUNDS;

        for r in 0..HALF_N_FULL_ROUNDS {
            <F as Poseidon>::constant_layer_field(&mut state, round_ctr);
            for i in 0..POSEIDON_SPONGE_WIDTH {
                row[full_sbox_1(r, i)] = state[i];
                // Generate x^3 and x^6 for the SBox layer constraints.
                row[reg_cubed_full(HALF_N_FULL_ROUNDS + r, i)] = state[i] * state[i] * state[i];
                row[reg_power_6_full(HALF_N_FULL_ROUNDS + r, i)] = row
                    [reg_cubed_full(HALF_N_FULL_ROUNDS + r, i)]
                    * row[reg_cubed_full(HALF_N_FULL_ROUNDS + r, i)];
                state[i] *= row[reg_power_6_full(HALF_N_FULL_ROUNDS + r, i)];
            }
            state = <F as Poseidon>::mds_layer_field(&state);
            round_ctr += 1;
        }

        for i in 0..POSEIDON_SPONGE_WIDTH {
            row[reg_output_limb(i)] = state[i];
        }

        row
    }

    pub fn generate_trace(
        &self,
        inputs: Vec<[u64; POSEIDON_SPONGE_WIDTH]>,
        min_rows: usize,
        timing: &mut TimingTree,
    ) -> Vec<PolynomialValues<F>> {
        // Generate the witness, except for permuted columns in the lookup argument.
        let trace_rows = timed!(
            timing,
            "generate trace rows",
            self.generate_trace_rows(inputs, min_rows)
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
        let lv = vars.get_local_values();

        // The filter must be 0 or 1.
        let filter = lv[FILTER];
        yield_constr.constraint(filter * (filter - P::ONES));

        // Compute the input layer. We assume that, when necessary,
        // input values were previously swapped before being passed
        // to Poseidon.
        let mut state = [P::ZEROS; POSEIDON_SPONGE_WIDTH];
        for i in 0..POSEIDON_SPONGE_WIDTH {
            state[i] = lv[reg_input_limb(i)];
        }

        let mut round_ctr = 0;

        // First set of full rounds.

        for r in 0..HALF_N_FULL_ROUNDS {
            <F as Poseidon>::constant_layer_packed_field(&mut state, round_ctr);

            for i in 0..POSEIDON_SPONGE_WIDTH {
                if r != 0 {
                    let sbox_in = lv[full_sbox_0(r, i)];
                    yield_constr.constraint(filter * (state[i] - sbox_in));
                    state[i] = sbox_in;
                }

                // Check that the powers were correctly generated.
                let cube = state[i] * state[i] * state[i];
                yield_constr.constraint(cube - lv[reg_cubed_full(r, i)]);
                let power_6 = lv[reg_cubed_full(r, i)] * lv[reg_cubed_full(r, i)];
                yield_constr.constraint(power_6 - lv[reg_power_6_full(r, i)]);
                state[i] *= lv[reg_power_6_full(r, i)];
            }

            state = <F as Poseidon>::mds_layer_packed_field(&state);
            round_ctr += 1;
        }

        // Partial rounds.
        <F as Poseidon>::partial_first_constant_layer_packed(&mut state);
        state = <F as Poseidon>::mds_partial_layer_packed_init(&state);
        for r in 0..(N_PARTIAL_ROUNDS - 1) {
            let sbox_in = lv[partial_sbox(r)];
            yield_constr.constraint(filter * (state[0] - sbox_in));
            state[0] = sbox_in;

            // Check that the powers were generated correctly.
            let cube = state[0] * state[0] * state[0];
            yield_constr.constraint(cube - lv[reg_cubed_partial(r)]);
            let power_6 = lv[reg_cubed_partial(r)] * lv[reg_cubed_partial(r)];
            yield_constr.constraint(power_6 - lv[reg_power_6_partial(r)]);

            state[0] = lv[reg_power_6_partial(r)] * sbox_in;
            state[0] +=
                P::Scalar::from_canonical_u64(<F as Poseidon>::FAST_PARTIAL_ROUND_CONSTANTS[r]);
            state = <F as Poseidon>::mds_partial_layer_fast_packed_field(&state, r);
        }
        let sbox_in = lv[partial_sbox(N_PARTIAL_ROUNDS - 1)];
        yield_constr.constraint(filter * (state[0] - sbox_in));
        state[0] = sbox_in;

        // Check that the powers were generated correctly.
        let cube = state[0] * state[0] * state[0];
        yield_constr.constraint(cube - lv[reg_cubed_partial(N_PARTIAL_ROUNDS - 1)]);
        let power_6 = lv[reg_cubed_partial(N_PARTIAL_ROUNDS - 1)]
            * lv[reg_cubed_partial(N_PARTIAL_ROUNDS - 1)];
        yield_constr.constraint(power_6 - lv[reg_power_6_partial(N_PARTIAL_ROUNDS - 1)]);

        state[0] = lv[reg_power_6_partial(N_PARTIAL_ROUNDS - 1)] * sbox_in;
        state = <F as Poseidon>::mds_partial_layer_fast_packed_field(&state, N_PARTIAL_ROUNDS - 1);
        round_ctr += N_PARTIAL_ROUNDS;

        // Second set of full rounds.
        for r in 0..HALF_N_FULL_ROUNDS {
            <F as Poseidon>::constant_layer_packed_field(&mut state, round_ctr);
            for i in 0..POSEIDON_SPONGE_WIDTH {
                let sbox_in = lv[full_sbox_1(r, i)];
                yield_constr.constraint(filter * (state[i] - sbox_in));
                state[i] = sbox_in;

                // Check that the powers were correctly generated.
                let cube = state[i] * state[i] * state[i];
                yield_constr.constraint(cube - lv[reg_cubed_full(HALF_N_FULL_ROUNDS + r, i)]);
                let power_6 = lv[reg_cubed_full(HALF_N_FULL_ROUNDS + r, i)]
                    * lv[reg_cubed_full(HALF_N_FULL_ROUNDS + r, i)];
                yield_constr.constraint(power_6 - lv[reg_power_6_full(HALF_N_FULL_ROUNDS + r, i)]);
                state[i] *= lv[reg_power_6_full(HALF_N_FULL_ROUNDS + r, i)];
            }
            state = <F as Poseidon>::mds_layer_packed_field(&state);
            round_ctr += 1;
        }

        for i in 0..POSEIDON_SPONGE_WIDTH {
            yield_constr.constraint(filter * (state[i] - lv[reg_output_limb(i)]));
        }
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: &Self::EvaluationFrameTarget,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let lv = vars.get_local_values();

        // The filter must be 0 or 1.
        let filter = lv[FILTER];
        let constr = builder.mul_sub_extension(filter, filter, filter);
        yield_constr.constraint(builder, constr);

        // Compute the input layer. We assume that, when necessary,
        // input values were previously swapped before being passed
        // to Poseidon.
        let mut state = [builder.zero_extension(); POSEIDON_SPONGE_WIDTH];
        for i in 0..POSEIDON_SPONGE_WIDTH {
            state[i] = lv[reg_input_limb(i)];
        }

        let mut round_ctr = 0;

        // First set of full rounds.
        for r in 0..HALF_N_FULL_ROUNDS {
            <F as Poseidon>::constant_layer_circuit(builder, &mut state, round_ctr);
            for i in 0..POSEIDON_SPONGE_WIDTH {
                if r != 0 {
                    let sbox_in = lv[full_sbox_0(r, i)];
                    let mut constr = builder.sub_extension(state[i], sbox_in);
                    constr = builder.mul_extension(filter, constr);
                    yield_constr.constraint(builder, constr);
                    state[i] = sbox_in;
                }

                // Check that the powers were correctly generated.
                let cube = builder.mul_many_extension([state[i], state[i], state[i]]);
                let constr = builder.sub_extension(cube, lv[reg_cubed_full(r, i)]);
                yield_constr.constraint(builder, constr);
                let power_6_constr = builder.mul_sub_extension(
                    lv[reg_cubed_full(r, i)],
                    lv[reg_cubed_full(r, i)],
                    lv[reg_power_6_full(r, i)],
                );
                yield_constr.constraint(builder, power_6_constr);

                // Update the i'th element of the state.
                state[i] = builder.mul_extension(state[i], lv[reg_power_6_full(r, i)]);
            }

            state = <F as Poseidon>::mds_layer_circuit(builder, &state);
            round_ctr += 1;
        }

        // Partial rounds.
        <F as Poseidon>::partial_first_constant_layer_circuit(builder, &mut state);
        state = <F as Poseidon>::mds_partial_layer_init_circuit(builder, &state);
        for r in 0..(N_PARTIAL_ROUNDS - 1) {
            let sbox_in = lv[partial_sbox(r)];
            let mut constr = builder.sub_extension(state[0], sbox_in);
            constr = builder.mul_extension(filter, constr);
            yield_constr.constraint(builder, constr);
            state[0] = sbox_in;

            // Check that the powers were generated correctly.
            let cube = builder.mul_many_extension([state[0], state[0], state[0]]);
            let constr = builder.sub_extension(cube, lv[reg_cubed_partial(r)]);
            yield_constr.constraint(builder, constr);
            let power_6_constr = builder.mul_sub_extension(
                lv[reg_cubed_partial(r)],
                lv[reg_cubed_partial(r)],
                lv[reg_power_6_partial(r)],
            );
            yield_constr.constraint(builder, power_6_constr);

            // Update state[0].
            state[0] = builder.mul_extension(lv[reg_power_6_partial(r)], sbox_in);
            state[0] = builder.add_const_extension(
                state[0],
                F::from_canonical_u64(<F as Poseidon>::FAST_PARTIAL_ROUND_CONSTANTS[r]),
            );
            state = <F as Poseidon>::mds_partial_layer_fast_circuit(builder, &state, r);
        }
        let sbox_in = lv[partial_sbox(N_PARTIAL_ROUNDS - 1)];
        let mut constr = builder.sub_extension(state[0], sbox_in);
        constr = builder.mul_extension(filter, constr);
        yield_constr.constraint(builder, constr);
        state[0] = sbox_in;

        // Check that the powers were generated correctly.
        let mut constr = builder.mul_many_extension([state[0], state[0], state[0]]);
        constr = builder.sub_extension(constr, lv[reg_cubed_partial(N_PARTIAL_ROUNDS - 1)]);
        yield_constr.constraint(builder, constr);
        let power_6_constr = builder.mul_sub_extension(
            lv[reg_cubed_partial(N_PARTIAL_ROUNDS - 1)],
            lv[reg_cubed_partial(N_PARTIAL_ROUNDS - 1)],
            lv[reg_power_6_partial(N_PARTIAL_ROUNDS - 1)],
        );
        yield_constr.constraint(builder, power_6_constr);

        state[0] = builder.mul_extension(lv[reg_power_6_partial(N_PARTIAL_ROUNDS - 1)], sbox_in);
        state =
            <F as Poseidon>::mds_partial_layer_fast_circuit(builder, &state, N_PARTIAL_ROUNDS - 1);
        round_ctr += N_PARTIAL_ROUNDS;

        // Second set of full rounds.
        for r in 0..HALF_N_FULL_ROUNDS {
            <F as Poseidon>::constant_layer_circuit(builder, &mut state, round_ctr);
            for i in 0..POSEIDON_SPONGE_WIDTH {
                let sbox_in = lv[full_sbox_1(r, i)];
                let mut constr = builder.sub_extension(state[i], sbox_in);
                constr = builder.mul_extension(filter, constr);
                yield_constr.constraint(builder, constr);
                state[i] = sbox_in;

                // Check that the powers were correctly generated.
                let mut constr = builder.mul_many_extension([state[i], state[i], state[i]]);
                constr =
                    builder.sub_extension(constr, lv[reg_cubed_full(HALF_N_FULL_ROUNDS + r, i)]);
                yield_constr.constraint(builder, constr);
                let power_6_constr = builder.mul_sub_extension(
                    lv[reg_cubed_full(HALF_N_FULL_ROUNDS + r, i)],
                    lv[reg_cubed_full(HALF_N_FULL_ROUNDS + r, i)],
                    lv[reg_power_6_full(HALF_N_FULL_ROUNDS + r, i)],
                );
                yield_constr.constraint(builder, power_6_constr);

                // Update the i'th element of the state.
                state[i] = builder
                    .mul_extension(state[i], lv[reg_power_6_full(HALF_N_FULL_ROUNDS + r, i)]);
            }

            state = <F as Poseidon>::mds_layer_circuit(builder, &state);
            round_ctr += 1;
        }

        for i in 0..POSEIDON_SPONGE_WIDTH {
            let mut constr = builder.sub_extension(state[i], lv[reg_output_limb(i)]);
            constr = builder.mul_extension(filter, constr);
            yield_constr.constraint(builder, constr);
        }
    }

    fn constraint_degree(&self) -> usize {
        3
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
    use plonky2::field::polynomial::PolynomialValues;
    use plonky2::field::types::{Field, PrimeField64};
    use plonky2::fri::oracle::PolynomialBatch;
    use plonky2::hash::poseidon::Poseidon;
    use plonky2::iop::challenger::Challenger;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::timed;
    use plonky2::util::timing::TimingTree;

    use crate::config::StarkConfig;
    use crate::cross_table_lookup::{
        CtlData, CtlZData, GrandProductChallenge, GrandProductChallengeSet,
    };
    use crate::poseidon::columns::{reg_output_limb, POSEIDON_SPONGE_WIDTH};
    use crate::poseidon::poseidon_stark::PoseidonStark;
    use crate::prover::prove_single_table;
    use crate::stark_testing::{test_stark_circuit_constraints, test_stark_low_degree};

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
        let input: [F; POSEIDON_SPONGE_WIDTH] = (0..POSEIDON_SPONGE_WIDTH)
            .map(|_| F::from_canonical_u64(rand::random()))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let int_inputs = input
            .iter()
            .map(|&inp| inp.to_canonical_u64())
            .collect::<Vec<u64>>()
            .try_into()
            .unwrap();
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = PoseidonStark<F, D>;

        let stark = S {
            f: Default::default(),
        };

        let rows = stark.generate_trace_rows(vec![int_inputs], 8);
        assert_eq!(rows.len(), 8);
        let last_row = rows[0];
        let output = &last_row[reg_output_limb(0)..reg_output_limb(POSEIDON_SPONGE_WIDTH - 1) + 1];

        let expected = <F as Poseidon>::poseidon(input);

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn poseidon_benchmark() -> Result<()> {
        const NUM_PERMS: usize = 85;
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = PoseidonStark<F, D>;
        let stark = S::default();
        let config = StarkConfig::standard_fast_config();

        init_logger();

        let input: Vec<[u64; POSEIDON_SPONGE_WIDTH]> = (0..NUM_PERMS)
            .map(|_| {
                (0..POSEIDON_SPONGE_WIDTH)
                    .map(|_| rand::random())
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap()
            })
            .collect();

        let mut timing = TimingTree::new("prove", log::Level::Debug);
        let trace_poly_values = timed!(
            timing,
            "generate trace",
            stark.generate_trace(input, 8, &mut timing)
        );

        // TODO: Cloning this isn't great; consider having `from_values` accept a reference,
        // or having `compute_permutation_z_polys` read trace values from the `PolynomialBatch`.
        let cloned_trace_poly_values = timed!(timing, "clone", trace_poly_values.clone());

        let trace_commitments = timed!(
            timing,
            "compute trace commitment",
            PolynomialBatch::<F, C, D>::from_values(
                cloned_trace_poly_values,
                config.fri_config.rate_bits,
                false,
                config.fri_config.cap_height,
                &mut timing,
                None,
            )
        );
        let degree = 1 << trace_commitments.degree_log;

        // Fake CTL data.
        let ctl_z_data = CtlZData {
            z: PolynomialValues::zero(degree),
            challenge: GrandProductChallenge {
                beta: F::ZERO,
                gamma: F::ZERO,
            },
            columns: vec![],
            filter_column: None,
        };
        let ctl_data = CtlData {
            zs_columns: vec![ctl_z_data.clone(); config.num_challenges],
        };

        prove_single_table(
            &stark,
            &config,
            &trace_poly_values,
            &trace_commitments,
            &ctl_data,
            &GrandProductChallengeSet {
                challenges: vec![ctl_z_data.challenge; config.num_challenges],
            },
            &mut Challenger::new(),
            &mut timing,
        )?;

        timing.print();
        Ok(())
    }

    fn init_logger() {
        let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "debug"));
    }
}
