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
    PoseidonSpongeColumnsView, NUM_POSEIDON_SPONGE_COLUMNS, POSEIDON_SPONGE_COL_MAP,
};
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cross_table_lookup::Column;
use crate::evaluation_frame::{StarkEvaluationFrame, StarkFrame};
use crate::poseidon::columns::{POSEIDON_SPONGE_RATE, POSEIDON_SPONGE_WIDTH};
use crate::poseidon_sponge::columns::NUM_DIGEST_ELEMENTS;
use crate::stark::Stark;
use crate::util::trace_rows_to_poly_values;
use crate::witness::memory::MemoryAddress;

pub(crate) fn ctl_looked_data<F: Field>() -> Vec<Column<F>> {
    let cols = POSEIDON_SPONGE_COL_MAP;
    // We only take the first NUM_DIGEST_ELEMENTS, divided into two 32-bits limb elements, for the digest.
    let outputs: Vec<_> = Column::singles(&cols.output_rate[0..2 * NUM_DIGEST_ELEMENTS]).collect();
    Column::singles([
        cols.context,
        cols.segment,
        cols.virt,
        cols.len,
        cols.timestamp,
    ])
    .chain(outputs)
    .collect()
}

pub(crate) fn ctl_looking<F: Field>() -> Vec<Column<F>> {
    let cols = POSEIDON_SPONGE_COL_MAP;
    let mut inputs: Vec<_> = (0..POSEIDON_SPONGE_RATE)
        .map(|i| {
            Column::linear_combination([
                (cols.state_rate[2 * i], F::ONE),
                (cols.state_rate[2 * i + 1], F::from_canonical_u64(1 << 32)),
            ])
        })
        .collect();
    inputs.extend(Column::singles(&cols.state_capacity));

    let mut outputs: Vec<_> = (0..POSEIDON_SPONGE_RATE)
        .map(|i| {
            Column::linear_combination([
                (cols.output_rate[2 * i], F::ONE),
                (cols.output_rate[2 * i + 1], F::from_canonical_u64(1 << 32)),
            ])
        })
        .collect();
    outputs.extend(Column::singles(&cols.output_capacity));

    let mut res = inputs;
    res.extend(outputs);

    res
}

pub(crate) fn num_logic_ctls() -> usize {
    POSEIDON_SPONGE_RATE
}

pub(crate) fn ctl_looking_logic<F: Field>(i: usize) -> Vec<Column<F>> {
    debug_assert!(i < POSEIDON_SPONGE_RATE);
    // We Xor the input of the next row with the output of the current row
    // to get the sponge state of the next row.
    let cols = POSEIDON_SPONGE_COL_MAP;
    let mut input = vec![Column::single_next_row(cols.block[i])];
    input.extend((0..7).map(|_| Column::zero()).collect::<Vec<Column<F>>>());

    let mut cur_state: Vec<Column<F>> =
        Column::singles([cols.output_rate[2 * i], cols.output_rate[2 * i + 1]]).collect();
    cur_state.extend((0..6).map(|_| Column::zero()).collect::<Vec<Column<F>>>());

    let mut xor_res: Vec<Column<F>> =
        Column::singles_next_row([cols.state_rate[2 * i], cols.state_rate[2 * i + 1]]).collect();
    xor_res.extend((0..6).map(|_| Column::zero()).collect::<Vec<Column<F>>>());
    let mut res = vec![Column::constant(F::from_canonical_usize(0x18))];
    res.extend(input);
    res.extend(cur_state);
    res.extend(xor_res);
    res
}

pub(crate) fn ctl_looking_memory<F: Field>(i: usize) -> Vec<Column<F>> {
    let cols = POSEIDON_SPONGE_COL_MAP;

    let mut res = vec![Column::constant(F::ONE)]; // is_read

    res.extend(Column::singles([cols.context, cols.segment]));

    // The address of the element read is `virt + already_absorbed_elements + i`.
    res.push(Column::linear_combination_with_constant(
        [
            (cols.virt, F::ONE),
            (cols.already_absorbed_elements, F::ONE),
        ],
        F::from_canonical_usize(i),
    ));

    // The i'th input element being read.
    res.push(Column::single(cols.block[i]));

    // We're reading a single element, so the higher limbs are 0.
    res.extend((1..8).map(|_| Column::zero()));

    res.push(Column::single(cols.timestamp));

    res
}

pub(crate) fn ctl_looked_filter<F: Field>() -> Column<F> {
    // The CPU table is only interested in our final-block rows, since those contain the final
    // sponge output.
    Column::sum(POSEIDON_SPONGE_COL_MAP.is_final_input_len)
}

pub(crate) fn ctl_looking_filter<F: Field>() -> Column<F> {
    let cols = POSEIDON_SPONGE_COL_MAP;
    Column::sum(once(&cols.is_full_input_block).chain(&cols.is_final_input_len))
}

pub(crate) fn ctl_logic_looking_filter<F: Field>() -> Column<F> {
    Column::single(POSEIDON_SPONGE_COL_MAP.is_full_input_block)
}

/// CTL filter for reading the `i`th input element from memory.
pub(crate) fn ctl_looking_memory_filter<F: Field>(i: usize) -> Column<F> {
    // We perform the `i`th read if either
    // - this is a full input block, or
    // - this is a final block of length `i` or greater
    let cols = POSEIDON_SPONGE_COL_MAP;
    if i == POSEIDON_SPONGE_RATE - 1 {
        Column::single(cols.is_full_input_block)
    } else {
        Column::sum(once(&cols.is_full_input_block).chain(&cols.is_final_input_len[i + 1..]))
    }
}
#[derive(Clone, Debug)]
pub(crate) struct PoseidonSpongeOp {
    /// The base address at which inputs are read.
    pub(crate) base_address: MemoryAddress,

    /// The timestamp at which inputs are read.
    pub(crate) timestamp: usize,

    /// The input that was read.
    pub(crate) input: Vec<u64>,
}

#[derive(Copy, Clone, Default)]
pub struct PoseidonSpongeStark<F, const D: usize> {
    f: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> PoseidonSpongeStark<F, D> {
    pub(crate) fn generate_trace(
        &self,
        operations: Vec<PoseidonSpongeOp>,
        min_rows: usize,
        timing: &mut TimingTree,
    ) -> Vec<PolynomialValues<F>> {
        // Generate the witness row-wise.
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

    fn generate_trace_rows(
        &self,
        operations: Vec<PoseidonSpongeOp>,
        min_rows: usize,
    ) -> Vec<[F; NUM_POSEIDON_SPONGE_COLUMNS]> {
        let base_len: usize = operations
            .iter()
            .map(|op| op.input.len() / POSEIDON_SPONGE_RATE + 1)
            .sum();
        let mut rows = Vec::with_capacity(base_len.max(min_rows).next_power_of_two());
        for op in operations {
            rows.extend(self.generate_rows_for_op(op));
        }
        let padded_rows = rows.len().max(min_rows).next_power_of_two();

        for _ in rows.len()..padded_rows {
            rows.push(self.generate_padding_row());
        }
        rows
    }
    fn generate_rows_for_op(&self, op: PoseidonSpongeOp) -> Vec<[F; NUM_POSEIDON_SPONGE_COLUMNS]> {
        let mut rows = Vec::with_capacity(op.input.len() / POSEIDON_SPONGE_RATE + 1);

        let mut state = [F::ZERO; POSEIDON_SPONGE_WIDTH];
        // First, pad the input.
        let mut input = op.input.clone();
        let last_non_padding_elt = input.len() % POSEIDON_SPONGE_RATE;

        // pad10*1 rule
        if input.len() % POSEIDON_SPONGE_RATE == POSEIDON_SPONGE_RATE - 1 {
            // Both 1s are placed in the same element.
            input.push(1);
        } else {
            input.push(1);
            while (input.len() + 1) % POSEIDON_SPONGE_RATE != 0 {
                input.push(0);
            }
            input.push(1);
        }

        let mut input_blocks = input.chunks_exact(POSEIDON_SPONGE_RATE);
        let total_length = input_blocks.len();
        let mut already_absorbed_elements = 0;
        for (counter, block) in input_blocks.by_ref().enumerate() {
            for (s, &b) in state[0..POSEIDON_SPONGE_RATE].iter_mut().zip_eq(block) {
                // We are reading from memory, so we assert that all elements
                // we receive are at most 32 bits long.
                debug_assert!(b >> 32 == 0);
                *s = F::from_canonical_u64(b ^ s.to_canonical_u64());
            }

            let row = if counter == total_length - 1 {
                let tmp_row = self.generate_trace_final_row_for_perm(
                    block,
                    last_non_padding_elt,
                    state,
                    &op,
                    already_absorbed_elements,
                );
                already_absorbed_elements += last_non_padding_elt;
                tmp_row
            } else {
                let tmp_row =
                    self.generate_trace_row_for_perm(block, state, &op, already_absorbed_elements);
                already_absorbed_elements += POSEIDON_SPONGE_RATE;
                tmp_row
            };

            // Update the state with the output of the permutation.
            for i in 0..POSEIDON_SPONGE_RATE {
                state[i] = row.output_rate[2 * i]
                    + F::from_canonical_u64(row.output_rate[2 * i + 1].to_canonical_u64() << 32);
            }
            state[POSEIDON_SPONGE_RATE..POSEIDON_SPONGE_WIDTH]
                .iter_mut()
                .zip_eq(&row.output_capacity)
                .for_each(|(s, &d)| *s = d);
            rows.push(row.into());
        }

        rows
    }

    fn generate_trace_row_for_perm(
        &self,
        block: &[u64],
        input: [F; POSEIDON_SPONGE_WIDTH],
        op: &PoseidonSpongeOp,
        already_absorbed_elements: usize,
    ) -> PoseidonSpongeColumnsView<F> {
        let mut row = PoseidonSpongeColumnsView::default();

        row.is_full_input_block = F::ONE;
        Self::generate_commons(&mut row, block, input, op, already_absorbed_elements);

        row
    }

    fn generate_trace_final_row_for_perm(
        &self,
        block: &[u64],
        length: usize,
        input: [F; POSEIDON_SPONGE_WIDTH],
        op: &PoseidonSpongeOp,
        already_absorbed_elements: usize,
    ) -> PoseidonSpongeColumnsView<F> {
        let mut row = PoseidonSpongeColumnsView::default();

        row.is_final_input_len[length] = F::ONE;
        Self::generate_commons(&mut row, block, input, op, already_absorbed_elements);

        row
    }

    fn generate_commons(
        row: &mut PoseidonSpongeColumnsView<F>,
        block: &[u64],
        input: [F; POSEIDON_SPONGE_WIDTH],
        op: &PoseidonSpongeOp,
        already_absorbed_elements: usize,
    ) {
        row.context = F::from_canonical_usize(op.base_address.context);
        row.segment = F::from_canonical_usize(op.base_address.segment);
        row.virt = F::from_canonical_usize(op.base_address.virt);
        row.timestamp = F::from_canonical_usize(op.timestamp);
        row.len = F::from_canonical_usize(op.input.len());
        row.already_absorbed_elements = F::from_canonical_usize(already_absorbed_elements);

        let output = <F as Poseidon>::poseidon(input);

        for i in 0..POSEIDON_SPONGE_RATE {
            // Set the block limbs. The input elements are supposed to be 32-bit limbs.
            row.block[i] = F::from_canonical_u32(block[i] as u32);

            // Update the sponge state.
            row.state_rate[2 * i] = F::from_canonical_u32(input[i].to_canonical_u64() as u32);
            row.state_rate[2 * i + 1] =
                F::from_canonical_u32((input[i].to_canonical_u64() >> 32) as u32);

            // Update the first `POSEIDON_SPONGE_RATE` elements of the output.
            row.output_rate[2 * i] = F::from_canonical_u32(output[i].to_canonical_u64() as u32);
            row.output_rate[2 * i + 1] =
                F::from_canonical_u32((output[i].to_canonical_u64() >> 32) as u32);
        }

        // Set the remaining elements of the sponge state and the output.
        for i in 0..POSEIDON_SPONGE_WIDTH - POSEIDON_SPONGE_RATE {
            row.state_capacity[i] = input[POSEIDON_SPONGE_RATE + i];
            row.output_capacity[i] = output[POSEIDON_SPONGE_RATE + i]
        }
    }

    fn generate_padding_row(&self) -> [F; NUM_POSEIDON_SPONGE_COLUMNS] {
        // The defaukt instance has is_full_input_block = is_final_block = 0,
        // indicating that it's a dummy/padding row.
        PoseidonSpongeColumnsView::default().into()
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for PoseidonSpongeStark<F, D> {
    type EvaluationFrame<FE, P, const D2: usize> = StarkFrame<P, NUM_POSEIDON_SPONGE_COLUMNS>
    where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>;

    type EvaluationFrameTarget = StarkFrame<ExtensionTarget<D>, NUM_POSEIDON_SPONGE_COLUMNS>;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: &Self::EvaluationFrame<FE, P, D2>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        // Note: We check the state update at the end of a permutation thanks to a CTL
        // with the Logic Stark. For this, we check that the current
        // state is equal the the previous outpout Xored with the current block.

        let lv: &[P; NUM_POSEIDON_SPONGE_COLUMNS] = vars.get_local_values().try_into().unwrap();
        let lv: &PoseidonSpongeColumnsView<P> = lv.borrow();
        let nv: &[P; NUM_POSEIDON_SPONGE_COLUMNS] = vars.get_next_values().try_into().unwrap();
        let nv: &PoseidonSpongeColumnsView<P> = nv.borrow();

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
        // Also, already_absorbed_elements = 0.
        let already_absorbed_elements = lv.already_absorbed_elements;
        yield_constr.constraint_first_row(already_absorbed_elements);
        for i in 0..POSEIDON_SPONGE_RATE {
            yield_constr.constraint_first_row(lv.state_rate[2 * i] - lv.block[i]);
        }
        for i in 0..POSEIDON_SPONGE_WIDTH - POSEIDON_SPONGE_RATE {
            yield_constr.constraint_first_row(lv.state_capacity[i]);
        }

        // If this is a final row and there is an upcoming operation, then
        // we make the previous checks for next row's `already_absorbed_elements`
        // and the original sponge state.
        yield_constr.constraint_transition(is_final_block * nv.already_absorbed_elements);
        for i in 0..POSEIDON_SPONGE_RATE {
            yield_constr
                .constraint_transition(is_final_block * (nv.state_rate[2 * i] - nv.block[i]));
        }
        for i in 0..POSEIDON_SPONGE_WIDTH - POSEIDON_SPONGE_RATE {
            yield_constr.constraint_transition(is_final_block * (nv.state_capacity[i]));
        }

        // If this is a full-input block, the next row's address,
        // time and len must match as well as its timestamp.
        yield_constr.constraint_transition(is_full_input_block * (lv.context - nv.context));
        yield_constr.constraint_transition(is_full_input_block * (lv.segment - nv.segment));
        yield_constr.constraint_transition(is_full_input_block * (lv.virt - nv.virt));
        yield_constr.constraint_transition(is_full_input_block * (lv.timestamp - nv.timestamp));

        // If this is a full-input block, the next row's already_absorbed_elements should be ours plus `POSEIDON_SPONGE_RATE`.
        yield_constr.constraint_transition(
            is_full_input_block
                * (already_absorbed_elements
                    + P::from(FE::from_canonical_usize(POSEIDON_SPONGE_RATE))
                    - nv.already_absorbed_elements),
        );
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
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: &Self::EvaluationFrameTarget,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        // Note: We check the state update at the end of a permutation thanks to a CTL
        // with the Logic Stark. For this, we check that the current
        // state is equal the the previous outpout Xored with the current block.

        let lv: &[ExtensionTarget<D>; NUM_POSEIDON_SPONGE_COLUMNS] =
            vars.get_local_values().try_into().unwrap();
        let lv: &PoseidonSpongeColumnsView<ExtensionTarget<D>> = lv.borrow();
        let nv: &[ExtensionTarget<D>; NUM_POSEIDON_SPONGE_COLUMNS] =
            vars.get_next_values().try_into().unwrap();
        let nv: &PoseidonSpongeColumnsView<ExtensionTarget<D>> = nv.borrow();

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
        for i in 0..POSEIDON_SPONGE_RATE {
            let constr = builder.sub_extension(lv.state_rate[2 * i], lv.block[i]);
            yield_constr.constraint_first_row(builder, constr);
        }
        for i in 0..POSEIDON_SPONGE_WIDTH - POSEIDON_SPONGE_RATE {
            yield_constr.constraint_first_row(builder, lv.state_capacity[i]);
        }

        // If this is a final row and there is an upcoming operation, then
        // we make the previous checks for next row's `already_absorbed_elements`
        // and the original sponge state.
        let constr = builder.mul_extension(is_final_block, nv.already_absorbed_elements);
        yield_constr.constraint_transition(builder, constr);
        for i in 0..POSEIDON_SPONGE_RATE {
            let mut constr = builder.sub_extension(nv.state_rate[2 * i], nv.block[i]);
            constr = builder.mul_extension(is_final_block, constr);
            yield_constr.constraint_transition(builder, constr);
        }
        for i in 0..POSEIDON_SPONGE_WIDTH - POSEIDON_SPONGE_RATE {
            let constr = builder.mul_extension(is_final_block, nv.state_capacity[i]);
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

        // If this is a full-input block, the next row's already_absorbed_elements should be ours plus `POSEIDON_SPONGE_RATE`.
        let diff = builder.sub_extension(already_absorbed_elements, nv.already_absorbed_elements);
        let constr = builder.arithmetic_extension(
            F::ONE,
            F::from_canonical_usize(POSEIDON_SPONGE_RATE),
            diff,
            is_full_input_block,
            is_full_input_block,
        );
        yield_constr.constraint_transition(builder, constr);

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
    }

    fn constraint_degree(&self) -> usize {
        3
    }
}
#[cfg(test)]
mod tests {
    use std::borrow::Borrow;

    use anyhow::Result;
    use plonky2::field::types::Field;
    use plonky2::hash::poseidon::Poseidon;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    use crate::memory::segments::Segment;
    use crate::poseidon::columns::{POSEIDON_SPONGE_RATE, POSEIDON_SPONGE_WIDTH};
    use crate::poseidon_sponge::columns::{PoseidonSpongeColumnsView, NUM_DIGEST_ELEMENTS};
    use crate::poseidon_sponge::poseidon_sponge_stark::{PoseidonSpongeOp, PoseidonSpongeStark};
    use crate::stark_testing::{test_stark_circuit_constraints, test_stark_low_degree};
    use crate::witness::memory::MemoryAddress;

    #[test]
    fn test_stark_degree() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = PoseidonSpongeStark<F, D>;

        let stark = S::default();
        test_stark_low_degree(stark)
    }

    #[test]
    fn test_stark_circuit() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = PoseidonSpongeStark<F, D>;

        let stark = S::default();
        test_stark_circuit_constraints::<F, C, S, D>(stark)
    }

    #[test]
    fn test_generation() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = PoseidonSpongeStark<F, D>;

        let input = vec![1, 2, 3];

        let mut padded_input = input
            .iter()
            .map(|&elt| F::from_canonical_u64(elt))
            .collect::<Vec<F>>();
        padded_input.push(F::ONE);
        while (padded_input.len() + 1) % POSEIDON_SPONGE_RATE != 0 {
            padded_input.push(F::ZERO);
        }
        padded_input.push(F::ONE);
        padded_input.extend(vec![F::ZERO; POSEIDON_SPONGE_WIDTH - POSEIDON_SPONGE_RATE]);

        let expected_output = <F as Poseidon>::poseidon(padded_input.try_into().unwrap());

        let op = PoseidonSpongeOp {
            base_address: MemoryAddress {
                context: 0,
                segment: Segment::Code as usize,
                virt: 0,
            },
            timestamp: 0,
            input,
        };

        let stark = S::default();
        let rows = stark.generate_rows_for_op(op);

        assert_eq!(rows.len(), 1);
        let last_row: &PoseidonSpongeColumnsView<F> = rows.last().unwrap().borrow();
        let mut output = [F::ZERO; NUM_DIGEST_ELEMENTS];
        for i in 0..NUM_DIGEST_ELEMENTS {
            output[i] = last_row.output_rate[2 * i]
                + last_row.output_rate[2 * i + 1] * F::from_canonical_u64(1 << 32);
        }

        assert_eq!(output, expected_output[0..NUM_DIGEST_ELEMENTS]);
        Ok(())
    }
}
