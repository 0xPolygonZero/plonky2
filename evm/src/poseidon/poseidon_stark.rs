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
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cross_table_lookup::Column;
use crate::evaluation_frame::{StarkEvaluationFrame, StarkFrame};
use crate::stark::Stark;
use crate::util::trace_rows_to_poly_values;
use crate::witness::memory::MemoryAddress;

pub fn ctl_looked_data<F: Field>() -> Vec<Column<F>> {
    let cols = POSEIDON_COL_MAP;
    let outputs: Vec<Column<F>> = Column::singles(cols.digest).collect();
    let mut res: Vec<_> = Column::singles([
        cols.context,
        cols.segment,
        cols.virt,
        cols.len,
        cols.timestamp,
    ])
    .collect();
    res.extend(outputs);
    res
}

pub fn ctl_looked_filter<F: Field>() -> Column<F> {
    Column::sum(POSEIDON_COL_MAP.is_final_input_len)
}

pub fn ctl_looking_memory<F: Field>(i: usize) -> Vec<Column<F>> {
    let cols = POSEIDON_COL_MAP;
    let mut res = vec![Column::constant(F::ONE)]; // is_read

    res.extend(Column::singles([cols.context, cols.segment]));

    res.push(Column::linear_combination_with_constant(
        [
            (cols.virt, F::ONE),
            (cols.already_absorbed_elements, F::ONE),
        ],
        F::from_canonical_usize(i),
    ));

    res.push(Column::single(cols.input[i]));
    res.extend((1..8).map(|_| Column::zero()));

    res.push(Column::single(cols.timestamp));

    assert_eq!(
        res.len(),
        crate::memory::memory_stark::ctl_data::<F>().len()
    );

    res
}

pub fn ctl_looking_memory_filter<F: Field>(i: usize) -> Column<F> {
    let cols = POSEIDON_COL_MAP;
    if i == POSEIDON_SPONGE_RATE - 1 {
        Column::single(cols.is_full_input_block)
    } else {
        Column::sum(once(&cols.is_full_input_block).chain(&cols.is_final_input_len[i + 1..]))
    }
}

#[derive(Clone, Debug)]
pub struct PoseidonOp {
    /// The base address at which inputs are read.
    pub(crate) base_address: MemoryAddress,

    /// The timestamp at which inputs are read.
    pub(crate) timestamp: usize,

    /// The input that was read. We assume that it was
    /// previously padded.
    pub(crate) input: Vec<u32>,

    /// Length of the input before paddding.
    pub(crate) len: usize,
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
        operations: Vec<PoseidonOp>,
        min_rows: usize,
    ) -> Vec<[F; NUM_COLUMNS]> {
        let base_len: usize = operations
            .iter()
            .map(|op| {
                debug_assert!(op.input.len() % POSEIDON_SPONGE_RATE == 0);
                op.input.len() / POSEIDON_SPONGE_RATE
            })
            .sum();

        let num_rows = base_len.max(min_rows).next_power_of_two();
        let mut rows = Vec::with_capacity(base_len.max(min_rows));

        for op in operations {
            rows.extend(self.generate_rows_for_op(op));
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
        while rows.len() < num_rows {
            rows.push(padding_row);
        }
        rows
    }

    fn generate_rows_for_op(&self, op: PoseidonOp) -> Vec<[F; NUM_COLUMNS]> {
        let mut input_blocks = op.input.chunks_exact(POSEIDON_SPONGE_RATE);
        let mut rows = Vec::with_capacity(op.input.len() / POSEIDON_SPONGE_RATE);
        let last_non_padding_elt = op.len % POSEIDON_SPONGE_RATE;
        let total_length = input_blocks.len();
        let mut already_absorbed_elements = 0;
        let mut state = [F::ZERO; POSEIDON_SPONGE_WIDTH];
        for (counter, block) in input_blocks.by_ref().enumerate() {
            for (s, &b) in state[0..POSEIDON_SPONGE_RATE].iter_mut().zip_eq(block) {
                *s = F::from_canonical_u32(b);
            }
            let row = if counter == total_length - 1 {
                let tmp_row =
                    self.generate_trace_final_row_for_perm(state, &op, already_absorbed_elements);
                already_absorbed_elements += last_non_padding_elt;
                tmp_row
            } else {
                let tmp_row =
                    self.generate_trace_row_for_perm(state, &op, already_absorbed_elements);
                already_absorbed_elements += POSEIDON_SPONGE_RATE;
                tmp_row
            };
            // Update state.
            for i in 0..POSEIDON_DIGEST {
                state[i] =
                    row.digest[2 * i] + F::from_canonical_u64(1 << 32) * row.digest[2 * i + 1];
            }
            state[POSEIDON_DIGEST..POSEIDON_SPONGE_WIDTH].copy_from_slice(&row.output_partial);

            rows.push(row.into());
        }
        rows
    }

    fn generate_commons(
        row: &mut PoseidonColumnsView<F>,
        input: [F; POSEIDON_SPONGE_WIDTH],
        op: &PoseidonOp,
        already_absorbed_elements: usize,
    ) {
        row.context = F::from_canonical_usize(op.base_address.context);
        row.segment = F::from_canonical_usize(op.base_address.segment);
        row.virt = F::from_canonical_usize(op.base_address.virt);
        row.timestamp = F::from_canonical_usize(op.timestamp);
        row.len = F::from_canonical_usize(op.len);
        row.already_absorbed_elements = F::from_canonical_usize(already_absorbed_elements);

        Self::generate_perm(row, input);
    }
    // One row per permutation.
    fn generate_trace_row_for_perm(
        &self,
        input: [F; POSEIDON_SPONGE_WIDTH],
        op: &PoseidonOp,
        already_absorbed_elements: usize,
    ) -> PoseidonColumnsView<F> {
        let mut row = PoseidonColumnsView::default();
        row.is_full_input_block = F::ONE;

        Self::generate_commons(&mut row, input, op, already_absorbed_elements);
        row
    }

    fn generate_trace_final_row_for_perm(
        &self,
        input: [F; POSEIDON_SPONGE_WIDTH],
        op: &PoseidonOp,
        already_absorbed_elements: usize,
    ) -> PoseidonColumnsView<F> {
        let mut row = PoseidonColumnsView::default();
        row.is_final_input_len[op.len % POSEIDON_SPONGE_RATE] = F::ONE;

        Self::generate_commons(&mut row, input, op, already_absorbed_elements);
        row
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
        operations: Vec<PoseidonOp>,
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
        let nv: &[P; NUM_COLUMNS] = vars.get_next_values().try_into().unwrap();
        let nv: &PoseidonColumnsView<P> = nv.borrow();

        // Each flag (full-input block, final block or implied dummy flag) must be boolean.
        let is_full_input_block = lv.is_full_input_block;
        yield_constr.constraint(is_full_input_block * (is_full_input_block - P::ONES));

        let is_final_block: P = lv.is_final_input_len.iter().copied().sum();
        yield_constr.constraint(is_final_block * (is_final_block - P::ONES));

        for &is_final_len in lv.is_final_input_len.iter() {
            yield_constr.constraint(is_final_len * (is_final_len - P::ONES));
        }

        // Ensure that full-input block and final block flags are not set to 1 at the same time.
        yield_constr.constraint(is_final_block * is_full_input_block);

        // If this is the first row, the original sponge state should have the input in the
        // first `POSEIDON_SPONGE_RATE` elements followed by 0 for the capacity elements.
        // The input values are checked with a CTL.
        // Also, already_absorbed_elements = 0.
        let already_absorbed_elements = lv.already_absorbed_elements;
        yield_constr.constraint_first_row(already_absorbed_elements);

        for i in POSEIDON_SPONGE_RATE..POSEIDON_SPONGE_WIDTH {
            yield_constr.constraint_first_row(lv.input[i]);
        }

        // If this is a final row and there is an upcoming operation, then
        // we make the previous checks for next row's `already_absorbed_elements`
        // and the original sponge state.
        yield_constr.constraint_transition(is_final_block * nv.already_absorbed_elements);

        for i in POSEIDON_SPONGE_RATE..POSEIDON_SPONGE_WIDTH {
            yield_constr.constraint_transition(is_final_block * nv.input[i]);
        }

        // If this is a full-input block, the next row's address,
        // time and len must match as well as its timestamp.
        yield_constr.constraint_transition(is_full_input_block * (lv.context - nv.context));
        yield_constr.constraint_transition(is_full_input_block * (lv.segment - nv.segment));
        yield_constr.constraint_transition(is_full_input_block * (lv.virt - nv.virt));
        yield_constr.constraint_transition(is_full_input_block * (lv.timestamp - nv.timestamp));

        // If this is a full-input block, the next row's already_absorbed_elements should be ours plus `POSEIDON_SPONGE_RATE`,
        // and the next input's capacity is the current output's capacity.
        yield_constr.constraint_transition(
            is_full_input_block
                * (already_absorbed_elements
                    + P::from(FE::from_canonical_usize(POSEIDON_SPONGE_RATE))
                    - nv.already_absorbed_elements),
        );

        for i in 0..POSEIDON_SPONGE_WIDTH - POSEIDON_SPONGE_RATE {
            yield_constr.constraint_transition(
                is_full_input_block
                    * (lv.output_partial[reg_output_capacity(i)]
                        - nv.input[POSEIDON_SPONGE_RATE + i]),
            );
        }

        // A dummy row is always followed by another dummy row, so the prover can't put dummy rows "in between" to avoid the above checks.
        let is_dummy = P::ONES - is_full_input_block - is_final_block;
        let next_is_final_block: P = nv.is_final_input_len.iter().copied().sum();
        yield_constr
            .constraint_transition(is_dummy * (nv.is_full_input_block + next_is_final_block));

        // If this is a final block, is_final_input_len implies `len - already_absorbed == i`.
        let offset = lv.len - already_absorbed_elements;
        for (i, &is_final_len) in lv.is_final_input_len.iter().enumerate() {
            let entry_match = offset - P::from(FE::from_canonical_usize(i));
            yield_constr.constraint(is_final_len * entry_match);
        }

        // Compute the input layer. We assume that, when necessary,
        // input values were previously swapped before being passed
        // to Poseidon.
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
        let nv: &[ExtensionTarget<D>; NUM_COLUMNS] = vars.get_next_values().try_into().unwrap();
        let nv: &PoseidonColumnsView<ExtensionTarget<D>> = nv.borrow();

        // Each flag (full-input block, final block or implied dummy flag) must be boolean.
        let is_full_input_block = lv.is_full_input_block;
        let constr = builder.mul_sub_extension(
            is_full_input_block,
            is_full_input_block,
            is_full_input_block,
        );
        yield_constr.constraint(builder, constr);

        let is_final_block = builder.add_many_extension(lv.is_final_input_len);
        let constr = builder.mul_sub_extension(is_final_block, is_final_block, is_final_block);
        yield_constr.constraint(builder, constr);

        for &is_final_len in lv.is_final_input_len.iter() {
            let constr = builder.mul_sub_extension(is_final_len, is_final_len, is_final_len);
            yield_constr.constraint(builder, constr);
        }

        // Ensure that full-input block and final block flags are not set to 1 at the same time.
        let constr = builder.mul_extension(is_final_block, is_full_input_block);
        yield_constr.constraint(builder, constr);

        // If this is the first row, the original sponge state should have the input in the
        // first `POSEIDON_SPONGE_RATE` elements followed by 0 for the capacity elements.
        // Also, already_absorbed_elements = 0.
        let already_absorbed_elements = lv.already_absorbed_elements;
        yield_constr.constraint_first_row(builder, already_absorbed_elements);

        for i in 0..POSEIDON_SPONGE_WIDTH - POSEIDON_SPONGE_RATE {
            yield_constr.constraint_first_row(builder, lv.input[reg_input_capacity(i)]);
        }

        // If this is a final row and there is an upcoming operation, then
        // we make the previous checks for next row's `already_absorbed_elements`
        // and the original sponge state.
        let constr = builder.mul_extension(is_final_block, nv.already_absorbed_elements);
        yield_constr.constraint_transition(builder, constr);

        for i in 0..POSEIDON_SPONGE_WIDTH - POSEIDON_SPONGE_RATE {
            let constr = builder.mul_extension(is_final_block, nv.input[reg_input_capacity(i)]);
            yield_constr.constraint_transition(builder, constr);
        }

        // If this is a full-input block, the next row's address,
        // time and len must match as well as its timestamp.
        let mut constr = builder.sub_extension(lv.context, nv.context);
        constr = builder.mul_extension(is_full_input_block, constr);
        yield_constr.constraint_transition(builder, constr);
        let mut constr = builder.sub_extension(lv.segment, nv.segment);
        constr = builder.mul_extension(is_full_input_block, constr);
        yield_constr.constraint_transition(builder, constr);
        let mut constr = builder.sub_extension(lv.virt, nv.virt);
        constr = builder.mul_extension(is_full_input_block, constr);
        yield_constr.constraint_transition(builder, constr);
        let mut constr = builder.sub_extension(lv.timestamp, nv.timestamp);
        constr = builder.mul_extension(is_full_input_block, constr);
        yield_constr.constraint_transition(builder, constr);

        // If this is a full-input block, the next row's already_absorbed_elements should be ours plus `POSEIDON_SPONGE_RATE`,
        // and the next input's capacity is the current output's capacity.
        let diff = builder.sub_extension(already_absorbed_elements, nv.already_absorbed_elements);
        let constr = builder.arithmetic_extension(
            F::ONE,
            F::from_canonical_usize(POSEIDON_SPONGE_RATE),
            diff,
            is_full_input_block,
            is_full_input_block,
        );
        yield_constr.constraint_transition(builder, constr);

        for i in 0..POSEIDON_SPONGE_WIDTH - POSEIDON_SPONGE_RATE {
            let mut constr = builder.sub_extension(
                lv.output_partial[reg_output_capacity(i)],
                nv.input[reg_input_capacity(i)],
            );
            constr = builder.mul_extension(is_full_input_block, constr);
            yield_constr.constraint_transition(builder, constr);
        }

        // A dummy row is always followed by another dummy row, so the prover can't put dummy rows "in between" to avoid the above checks.
        let mut is_dummy = builder.add_extension(is_full_input_block, is_final_block);
        let one = builder.one_extension();
        is_dummy = builder.sub_extension(one, is_dummy);
        let next_is_final_block = builder.add_many_extension(nv.is_final_input_len.iter());
        let mut constr = builder.add_extension(nv.is_full_input_block, next_is_final_block);
        constr = builder.mul_extension(is_dummy, constr);
        yield_constr.constraint_transition(builder, constr);

        // If this is a final block, is_final_input_len implies `len - already_absorbed == i`
        let offset = builder.sub_extension(lv.len, already_absorbed_elements);
        for (i, &is_final_len) in lv.is_final_input_len.iter().enumerate() {
            let index = builder.constant_extension(F::from_canonical_usize(i).into());
            let entry_match = builder.sub_extension(offset, index);
            let constr = builder.mul_extension(is_final_len, entry_match);
            yield_constr.constraint(builder, constr);
        }

        // Compute the input layer. We assume that, when necessary,
        // input values were previously swapped before being passed
        // to Poseidon.
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
        let input: [F; POSEIDON_SPONGE_RATE] = (0..POSEIDON_SPONGE_RATE)
            .map(|_| F::from_canonical_u32(rand::random()))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let int_inputs = PoseidonOp {
            base_address: MemoryAddress::new(
                0,
                crate::memory::segments::Segment::AccessedAddresses,
                0,
            ),
            input: input
                .iter()
                .map(|&inp| inp.to_canonical_u64() as u32)
                .collect::<Vec<u32>>(),
            timestamp: 0,
            len: POSEIDON_SPONGE_RATE,
        };
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = PoseidonStark<F, D>;

        let stark = S {
            f: Default::default(),
        };

        let rows = stark.generate_trace_rows(vec![int_inputs], 8);
        assert_eq!(rows.len(), 8);
        let last_row: &PoseidonColumnsView<F> = rows[0].borrow();
        let mut output: Vec<_> = (0..POSEIDON_DIGEST)
            .map(|i| {
                last_row.digest[2 * i] + F::from_canonical_u64(1 << 32) * last_row.digest[2 * i + 1]
            })
            .collect();
        output.extend(&last_row.output_partial);

        let mut state = input.to_vec();
        state.extend(vec![F::ZERO; POSEIDON_SPONGE_WIDTH - POSEIDON_SPONGE_RATE]);
        let expected = <F as Poseidon>::poseidon(state.try_into().unwrap());

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

        let input: Vec<Vec<u32>> = (0..NUM_PERMS)
            .map(|_| {
                (0..POSEIDON_SPONGE_RATE)
                    .map(|_| rand::random())
                    .collect::<Vec<_>>()
            })
            .collect();
        let ops: Vec<_> = (0..NUM_PERMS)
            .map(|i| PoseidonOp {
                base_address: MemoryAddress::new(0, Segment::BlockHashes, 0),
                timestamp: 0,
                input: input[i].clone(),
                len: 5,
            })
            .collect();
        let mut timing = TimingTree::new("prove", log::Level::Debug);
        let trace_poly_values = timed!(
            timing,
            "generate trace",
            stark.generate_trace(ops, 8, &mut timing)
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
