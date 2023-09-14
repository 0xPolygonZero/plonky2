use std::borrow::Borrow;
use std::iter::{once, repeat};
use std::marker::PhantomData;
use std::mem::size_of;

use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::timed;
use plonky2::util::timing::TimingTree;
use plonky2_util::ceil_div_usize;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::kernel::keccak_util::keccakf_u32s;
use crate::cross_table_lookup::Column;
use crate::keccak_sponge::columns::*;
use crate::stark::Stark;
use crate::util::trace_rows_to_poly_values;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};
use crate::witness::memory::MemoryAddress;

pub(crate) fn ctl_looked_data<F: Field>() -> Vec<Column<F>> {
    let cols = KECCAK_SPONGE_COL_MAP;
    let mut outputs = Vec::with_capacity(8);
    for i in (0..8).rev() {
        let cur_col = Column::linear_combination(
            cols.updated_digest_state_bytes[i * 4..(i + 1) * 4]
                .iter()
                .enumerate()
                .map(|(j, &c)| (c, F::from_canonical_u64(1 << (24 - 8 * j)))),
        );
        outputs.push(cur_col);
    }

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

pub(crate) fn ctl_looking_keccak<F: Field>() -> Vec<Column<F>> {
    let cols = KECCAK_SPONGE_COL_MAP;
    let mut res: Vec<_> = Column::singles(
        [
            cols.xored_rate_u32s.as_slice(),
            &cols.original_capacity_u32s,
        ]
        .concat(),
    )
    .collect();

    // We recover the 32-bit digest limbs from their corresponding bytes,
    // and then append them to the rest of the updated state limbs.
    let digest_u32s = cols.updated_digest_state_bytes.chunks_exact(4).map(|c| {
        Column::linear_combination(
            c.iter()
                .enumerate()
                .map(|(i, &b)| (b, F::from_canonical_usize(1 << (8 * i)))),
        )
    });

    res.extend(digest_u32s);

    res.extend(Column::singles(&cols.partial_updated_state_u32s));

    res
}

pub(crate) fn ctl_looking_memory<F: Field>(i: usize) -> Vec<Column<F>> {
    let cols = KECCAK_SPONGE_COL_MAP;

    let mut res = vec![Column::constant(F::ONE)]; // is_read

    res.extend(Column::singles([cols.context, cols.segment]));

    // The address of the byte being read is `virt + already_absorbed_bytes + i`.
    res.push(Column::linear_combination_with_constant(
        [(cols.virt, F::ONE), (cols.already_absorbed_bytes, F::ONE)],
        F::from_canonical_usize(i),
    ));

    // The i'th input byte being read.
    res.push(Column::single(cols.block_bytes[i]));

    // Since we're reading a single byte, the higher limbs must be zero.
    res.extend((1..8).map(|_| Column::zero()));

    res.push(Column::single(cols.timestamp));

    assert_eq!(
        res.len(),
        crate::memory::memory_stark::ctl_data::<F>().len()
    );
    res
}

pub(crate) fn num_logic_ctls() -> usize {
    const U8S_PER_CTL: usize = 32;
    ceil_div_usize(KECCAK_RATE_BYTES, U8S_PER_CTL)
}

/// CTL for performing the `i`th logic CTL. Since we need to do 136 byte XORs, and the logic CTL can
/// XOR 32 bytes per CTL, there are 5 such CTLs.
pub(crate) fn ctl_looking_logic<F: Field>(i: usize) -> Vec<Column<F>> {
    const U32S_PER_CTL: usize = 8;
    const U8S_PER_CTL: usize = 32;

    debug_assert!(i < num_logic_ctls());
    let cols = KECCAK_SPONGE_COL_MAP;

    let mut res = vec![
        Column::constant(F::from_canonical_u8(0x18)), // is_xor
    ];

    // Input 0 contains some of the sponge's original rate chunks. If this is the last CTL, we won't
    // need to use all of the CTL's inputs, so we will pass some zeros.
    res.extend(
        Column::singles(&cols.original_rate_u32s[i * U32S_PER_CTL..])
            .chain(repeat(Column::zero()))
            .take(U32S_PER_CTL),
    );

    // Input 1 contains some of block's chunks. Again, for the last CTL it will include some zeros.
    res.extend(
        cols.block_bytes[i * U8S_PER_CTL..]
            .chunks(size_of::<u32>())
            .map(|chunk| Column::le_bytes(chunk))
            .chain(repeat(Column::zero()))
            .take(U32S_PER_CTL),
    );

    // The output contains the XOR'd rate part.
    res.extend(
        Column::singles(&cols.xored_rate_u32s[i * U32S_PER_CTL..])
            .chain(repeat(Column::zero()))
            .take(U32S_PER_CTL),
    );

    res
}

pub(crate) fn ctl_looked_filter<F: Field>() -> Column<F> {
    // The CPU table is only interested in our final-block rows, since those contain the final
    // sponge output.
    Column::sum(KECCAK_SPONGE_COL_MAP.is_final_input_len)
}

/// CTL filter for reading the `i`th byte of input from memory.
pub(crate) fn ctl_looking_memory_filter<F: Field>(i: usize) -> Column<F> {
    // We perform the `i`th read if either
    // - this is a full input block, or
    // - this is a final block of length `i` or greater
    let cols = KECCAK_SPONGE_COL_MAP;
    if i == KECCAK_RATE_BYTES - 1 {
        Column::single(cols.is_full_input_block)
    } else {
        Column::sum(once(&cols.is_full_input_block).chain(&cols.is_final_input_len[i + 1..]))
    }
}

/// CTL filter for looking at XORs in the logic table.
pub(crate) fn ctl_looking_logic_filter<F: Field>() -> Column<F> {
    let cols = KECCAK_SPONGE_COL_MAP;
    Column::sum(once(&cols.is_full_input_block).chain(&cols.is_final_input_len))
}

pub(crate) fn ctl_looking_keccak_filter<F: Field>() -> Column<F> {
    let cols = KECCAK_SPONGE_COL_MAP;
    Column::sum(once(&cols.is_full_input_block).chain(&cols.is_final_input_len))
}

/// Information about a Keccak sponge operation needed for witness generation.
#[derive(Clone, Debug)]
pub(crate) struct KeccakSpongeOp {
    /// The base address at which inputs are read.
    pub(crate) base_address: MemoryAddress,

    /// The timestamp at which inputs are read.
    pub(crate) timestamp: usize,

    /// The input that was read.
    pub(crate) input: Vec<u8>,
}

#[derive(Copy, Clone, Default)]
pub struct KeccakSpongeStark<F, const D: usize> {
    f: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> KeccakSpongeStark<F, D> {
    pub(crate) fn generate_trace(
        &self,
        operations: Vec<KeccakSpongeOp>,
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
        operations: Vec<KeccakSpongeOp>,
        min_rows: usize,
    ) -> Vec<[F; NUM_KECCAK_SPONGE_COLUMNS]> {
        let base_len: usize = operations
            .iter()
            .map(|op| op.input.len() / KECCAK_RATE_BYTES + 1)
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

    fn generate_rows_for_op(&self, op: KeccakSpongeOp) -> Vec<[F; NUM_KECCAK_SPONGE_COLUMNS]> {
        let mut rows = Vec::with_capacity(op.input.len() / KECCAK_RATE_BYTES + 1);

        let mut sponge_state = [0u32; KECCAK_WIDTH_U32S];

        let mut input_blocks = op.input.chunks_exact(KECCAK_RATE_BYTES);
        let mut already_absorbed_bytes = 0;
        for block in input_blocks.by_ref() {
            let row = self.generate_full_input_row(
                &op,
                already_absorbed_bytes,
                sponge_state,
                block.try_into().unwrap(),
            );

            sponge_state[..KECCAK_DIGEST_U32S]
                .iter_mut()
                .zip(row.updated_digest_state_bytes.chunks_exact(4))
                .for_each(|(s, bs)| {
                    *s = bs
                        .iter()
                        .enumerate()
                        .map(|(i, b)| (b.to_canonical_u64() as u32) << (8 * i))
                        .sum();
                });

            sponge_state[KECCAK_DIGEST_U32S..]
                .iter_mut()
                .zip(row.partial_updated_state_u32s)
                .for_each(|(s, x)| *s = x.to_canonical_u64() as u32);

            rows.push(row.into());
            already_absorbed_bytes += KECCAK_RATE_BYTES;
        }

        rows.push(
            self.generate_final_row(
                &op,
                already_absorbed_bytes,
                sponge_state,
                input_blocks.remainder(),
            )
            .into(),
        );

        rows
    }

    fn generate_full_input_row(
        &self,
        op: &KeccakSpongeOp,
        already_absorbed_bytes: usize,
        sponge_state: [u32; KECCAK_WIDTH_U32S],
        block: [u8; KECCAK_RATE_BYTES],
    ) -> KeccakSpongeColumnsView<F> {
        let mut row = KeccakSpongeColumnsView {
            is_full_input_block: F::ONE,
            ..Default::default()
        };

        row.block_bytes = block.map(F::from_canonical_u8);

        Self::generate_common_fields(&mut row, op, already_absorbed_bytes, sponge_state);
        row
    }

    fn generate_final_row(
        &self,
        op: &KeccakSpongeOp,
        already_absorbed_bytes: usize,
        sponge_state: [u32; KECCAK_WIDTH_U32S],
        final_inputs: &[u8],
    ) -> KeccakSpongeColumnsView<F> {
        assert_eq!(already_absorbed_bytes + final_inputs.len(), op.input.len());

        let mut row = KeccakSpongeColumnsView::default();

        for (block_byte, input_byte) in row.block_bytes.iter_mut().zip(final_inputs) {
            *block_byte = F::from_canonical_u8(*input_byte);
        }

        // pad10*1 rule
        if final_inputs.len() == KECCAK_RATE_BYTES - 1 {
            // Both 1s are placed in the same byte.
            row.block_bytes[final_inputs.len()] = F::from_canonical_u8(0b10000001);
        } else {
            row.block_bytes[final_inputs.len()] = F::ONE;
            row.block_bytes[KECCAK_RATE_BYTES - 1] = F::from_canonical_u8(0b10000000);
        }

        row.is_final_input_len[final_inputs.len()] = F::ONE;

        Self::generate_common_fields(&mut row, op, already_absorbed_bytes, sponge_state);
        row
    }

    /// Generate fields that are common to both full-input-block rows and final-block rows.
    /// Also updates the sponge state with a single absorption.
    fn generate_common_fields(
        row: &mut KeccakSpongeColumnsView<F>,
        op: &KeccakSpongeOp,
        already_absorbed_bytes: usize,
        mut sponge_state: [u32; KECCAK_WIDTH_U32S],
    ) {
        row.context = F::from_canonical_usize(op.base_address.context);
        row.segment = F::from_canonical_usize(op.base_address.segment);
        row.virt = F::from_canonical_usize(op.base_address.virt);
        row.timestamp = F::from_canonical_usize(op.timestamp);
        row.len = F::from_canonical_usize(op.input.len());
        row.already_absorbed_bytes = F::from_canonical_usize(already_absorbed_bytes);

        row.original_rate_u32s = sponge_state[..KECCAK_RATE_U32S]
            .iter()
            .map(|x| F::from_canonical_u32(*x))
            .collect_vec()
            .try_into()
            .unwrap();

        row.original_capacity_u32s = sponge_state[KECCAK_RATE_U32S..]
            .iter()
            .map(|x| F::from_canonical_u32(*x))
            .collect_vec()
            .try_into()
            .unwrap();

        let block_u32s = (0..KECCAK_RATE_U32S).map(|i| {
            u32::from_le_bytes(
                row.block_bytes[i * 4..(i + 1) * 4]
                    .iter()
                    .map(|x| x.to_canonical_u64() as u8)
                    .collect_vec()
                    .try_into()
                    .unwrap(),
            )
        });

        // xor in the block
        for (state_i, block_i) in sponge_state.iter_mut().zip(block_u32s) {
            *state_i ^= block_i;
        }
        let xored_rate_u32s: [u32; KECCAK_RATE_U32S] = sponge_state[..KECCAK_RATE_U32S]
            .to_vec()
            .try_into()
            .unwrap();
        row.xored_rate_u32s = xored_rate_u32s.map(F::from_canonical_u32);

        keccakf_u32s(&mut sponge_state);
        // Store all but the first `KECCAK_DIGEST_U32S` limbs in the updated state.
        // Those missing limbs will be broken down into bytes and stored separately.
        row.partial_updated_state_u32s.copy_from_slice(
            &sponge_state[KECCAK_DIGEST_U32S..]
                .iter()
                .copied()
                .map(|i| F::from_canonical_u32(i))
                .collect::<Vec<_>>(),
        );
        sponge_state[..KECCAK_DIGEST_U32S]
            .iter()
            .enumerate()
            .for_each(|(l, &elt)| {
                let mut cur_elt = elt;
                (0..4).for_each(|i| {
                    row.updated_digest_state_bytes[l * 4 + i] =
                        F::from_canonical_u32(cur_elt & 0xFF);
                    cur_elt >>= 8;
                });

                // 32-bit limb reconstruction consistency check.
                let mut s = row.updated_digest_state_bytes[l * 4].to_canonical_u64();
                for i in 1..4 {
                    s += row.updated_digest_state_bytes[l * 4 + i].to_canonical_u64() << (8 * i);
                }
                assert_eq!(elt as u64, s, "not equal");
            })
    }

    fn generate_padding_row(&self) -> [F; NUM_KECCAK_SPONGE_COLUMNS] {
        // The default instance has is_full_input_block = is_final_block = 0,
        // indicating that it's a dummy/padding row.
        KeccakSpongeColumnsView::default().into()
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for KeccakSpongeStark<F, D> {
    const COLUMNS: usize = NUM_KECCAK_SPONGE_COLUMNS;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        let local_values: &KeccakSpongeColumnsView<P> = vars.local_values.borrow();
        let next_values: &KeccakSpongeColumnsView<P> = vars.next_values.borrow();

        // Each flag (full-input block, final block or implied dummy flag) must be boolean.
        let is_full_input_block = local_values.is_full_input_block;
        yield_constr.constraint(is_full_input_block * (is_full_input_block - P::ONES));

        let is_final_block: P = local_values.is_final_input_len.iter().copied().sum();
        yield_constr.constraint(is_final_block * (is_final_block - P::ONES));

        for &is_final_len in local_values.is_final_input_len.iter() {
            yield_constr.constraint(is_final_len * (is_final_len - P::ONES));
        }

        // Ensure that full-input block and final block flags are not set to 1 at the same time.
        yield_constr.constraint(is_final_block * is_full_input_block);

        // If this is the first row, the original sponge state should be 0 and already_absorbed_bytes = 0.
        let already_absorbed_bytes = local_values.already_absorbed_bytes;
        yield_constr.constraint_first_row(already_absorbed_bytes);
        for &original_rate_elem in local_values.original_rate_u32s.iter() {
            yield_constr.constraint_first_row(original_rate_elem);
        }
        for &original_capacity_elem in local_values.original_capacity_u32s.iter() {
            yield_constr.constraint_first_row(original_capacity_elem);
        }

        // If this is a final block, the next row's original sponge state should be 0 and already_absorbed_bytes = 0.
        yield_constr.constraint_transition(is_final_block * next_values.already_absorbed_bytes);
        for &original_rate_elem in next_values.original_rate_u32s.iter() {
            yield_constr.constraint_transition(is_final_block * original_rate_elem);
        }
        for &original_capacity_elem in next_values.original_capacity_u32s.iter() {
            yield_constr.constraint_transition(is_final_block * original_capacity_elem);
        }

        // If this is a full-input block, the next row's address, time and len must match as well as its timestamp.
        yield_constr.constraint_transition(
            is_full_input_block * (local_values.context - next_values.context),
        );
        yield_constr.constraint_transition(
            is_full_input_block * (local_values.segment - next_values.segment),
        );
        yield_constr
            .constraint_transition(is_full_input_block * (local_values.virt - next_values.virt));
        yield_constr.constraint_transition(
            is_full_input_block * (local_values.timestamp - next_values.timestamp),
        );

        // If this is a full-input block, the next row's "before" should match our "after" state.
        for (current_bytes_after, next_before) in local_values
            .updated_digest_state_bytes
            .chunks_exact(4)
            .zip(&next_values.original_rate_u32s[..KECCAK_DIGEST_U32S])
        {
            let mut current_after = current_bytes_after[0];
            for i in 1..4 {
                current_after +=
                    current_bytes_after[i] * P::from(FE::from_canonical_usize(1 << (8 * i)));
            }
            yield_constr
                .constraint_transition(is_full_input_block * (*next_before - current_after));
        }
        for (&current_after, &next_before) in local_values
            .partial_updated_state_u32s
            .iter()
            .zip(next_values.original_rate_u32s[KECCAK_DIGEST_U32S..].iter())
        {
            yield_constr.constraint_transition(is_full_input_block * (next_before - current_after));
        }
        for (&current_after, &next_before) in local_values
            .partial_updated_state_u32s
            .iter()
            .skip(KECCAK_RATE_U32S - KECCAK_DIGEST_U32S)
            .zip(next_values.original_capacity_u32s.iter())
        {
            yield_constr.constraint_transition(is_full_input_block * (next_before - current_after));
        }

        // If this is a full-input block, the next row's already_absorbed_bytes should be ours plus `KECCAK_RATE_BYTES`.
        yield_constr.constraint_transition(
            is_full_input_block
                * (already_absorbed_bytes + P::from(FE::from_canonical_usize(KECCAK_RATE_BYTES))
                    - next_values.already_absorbed_bytes),
        );

        // A dummy row is always followed by another dummy row, so the prover can't put dummy rows "in between" to avoid the above checks.
        let is_dummy = P::ONES - is_full_input_block - is_final_block;
        let next_is_final_block: P = next_values.is_final_input_len.iter().copied().sum();
        yield_constr.constraint_transition(
            is_dummy * (next_values.is_full_input_block + next_is_final_block),
        );

        // If this is a final block, is_final_input_len implies `len - already_absorbed == i`.
        let offset = local_values.len - already_absorbed_bytes;
        for (i, &is_final_len) in local_values.is_final_input_len.iter().enumerate() {
            let entry_match = offset - P::from(FE::from_canonical_usize(i));
            yield_constr.constraint(is_final_len * entry_match);
        }
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let local_values: &KeccakSpongeColumnsView<ExtensionTarget<D>> = vars.local_values.borrow();
        let next_values: &KeccakSpongeColumnsView<ExtensionTarget<D>> = vars.next_values.borrow();

        let one = builder.one_extension();

        // Each flag (full-input block, final block or implied dummy flag) must be boolean.
        let is_full_input_block = local_values.is_full_input_block;
        let constraint = builder.mul_sub_extension(
            is_full_input_block,
            is_full_input_block,
            is_full_input_block,
        );
        yield_constr.constraint(builder, constraint);

        let is_final_block = builder.add_many_extension(local_values.is_final_input_len);
        let constraint = builder.mul_sub_extension(is_final_block, is_final_block, is_final_block);
        yield_constr.constraint(builder, constraint);

        for &is_final_len in local_values.is_final_input_len.iter() {
            let constraint = builder.mul_sub_extension(is_final_len, is_final_len, is_final_len);
            yield_constr.constraint(builder, constraint);
        }

        // Ensure that full-input block and final block flags are not set to 1 at the same time.
        let constraint = builder.mul_extension(is_final_block, is_full_input_block);
        yield_constr.constraint(builder, constraint);

        // If this is the first row, the original sponge state should be 0 and already_absorbed_bytes = 0.
        let already_absorbed_bytes = local_values.already_absorbed_bytes;
        yield_constr.constraint_first_row(builder, already_absorbed_bytes);
        for &original_rate_elem in local_values.original_rate_u32s.iter() {
            yield_constr.constraint_first_row(builder, original_rate_elem);
        }
        for &original_capacity_elem in local_values.original_capacity_u32s.iter() {
            yield_constr.constraint_first_row(builder, original_capacity_elem);
        }

        // If this is a final block, the next row's original sponge state should be 0 and already_absorbed_bytes = 0.
        let constraint = builder.mul_extension(is_final_block, next_values.already_absorbed_bytes);
        yield_constr.constraint_transition(builder, constraint);
        for &original_rate_elem in next_values.original_rate_u32s.iter() {
            let constraint = builder.mul_extension(is_final_block, original_rate_elem);
            yield_constr.constraint_transition(builder, constraint);
        }
        for &original_capacity_elem in next_values.original_capacity_u32s.iter() {
            let constraint = builder.mul_extension(is_final_block, original_capacity_elem);
            yield_constr.constraint_transition(builder, constraint);
        }

        // If this is a full-input block, the next row's address, time and len must match as well as its timestamp.
        let context_diff = builder.sub_extension(local_values.context, next_values.context);
        let constraint = builder.mul_extension(is_full_input_block, context_diff);
        yield_constr.constraint_transition(builder, constraint);

        let segment_diff = builder.sub_extension(local_values.segment, next_values.segment);
        let constraint = builder.mul_extension(is_full_input_block, segment_diff);
        yield_constr.constraint_transition(builder, constraint);

        let virt_diff = builder.sub_extension(local_values.virt, next_values.virt);
        let constraint = builder.mul_extension(is_full_input_block, virt_diff);
        yield_constr.constraint_transition(builder, constraint);

        let timestamp_diff = builder.sub_extension(local_values.timestamp, next_values.timestamp);
        let constraint = builder.mul_extension(is_full_input_block, timestamp_diff);
        yield_constr.constraint_transition(builder, constraint);

        // If this is a full-input block, the next row's "before" should match our "after" state.
        for (current_bytes_after, next_before) in local_values
            .updated_digest_state_bytes
            .chunks_exact(4)
            .zip(&next_values.original_rate_u32s[..KECCAK_DIGEST_U32S])
        {
            let mut current_after = current_bytes_after[0];
            for i in 1..4 {
                current_after = builder.mul_const_add_extension(
                    F::from_canonical_usize(1 << (8 * i)),
                    current_bytes_after[i],
                    current_after,
                );
            }
            let diff = builder.sub_extension(*next_before, current_after);
            let constraint = builder.mul_extension(is_full_input_block, diff);
            yield_constr.constraint_transition(builder, constraint);
        }
        for (&current_after, &next_before) in local_values
            .partial_updated_state_u32s
            .iter()
            .zip(next_values.original_rate_u32s[KECCAK_DIGEST_U32S..].iter())
        {
            let diff = builder.sub_extension(next_before, current_after);
            let constraint = builder.mul_extension(is_full_input_block, diff);
            yield_constr.constraint_transition(builder, constraint);
        }
        for (&current_after, &next_before) in local_values
            .partial_updated_state_u32s
            .iter()
            .skip(KECCAK_RATE_U32S - KECCAK_DIGEST_U32S)
            .zip(next_values.original_capacity_u32s.iter())
        {
            let diff = builder.sub_extension(next_before, current_after);
            let constraint = builder.mul_extension(is_full_input_block, diff);
            yield_constr.constraint_transition(builder, constraint);
        }

        // If this is a full-input block, the next row's already_absorbed_bytes should be ours plus `KECCAK_RATE_BYTES`.
        let absorbed_bytes = builder.add_const_extension(
            already_absorbed_bytes,
            F::from_canonical_usize(KECCAK_RATE_BYTES),
        );
        let absorbed_diff =
            builder.sub_extension(absorbed_bytes, next_values.already_absorbed_bytes);
        let constraint = builder.mul_extension(is_full_input_block, absorbed_diff);
        yield_constr.constraint_transition(builder, constraint);

        // A dummy row is always followed by another dummy row, so the prover can't put dummy rows "in between" to avoid the above checks.
        let is_dummy = {
            let tmp = builder.sub_extension(one, is_final_block);
            builder.sub_extension(tmp, is_full_input_block)
        };
        let next_is_final_block = builder.add_many_extension(next_values.is_final_input_len);
        let constraint = {
            let tmp = builder.add_extension(next_is_final_block, next_values.is_full_input_block);
            builder.mul_extension(is_dummy, tmp)
        };
        yield_constr.constraint_transition(builder, constraint);

        // If this is a final block, is_final_input_len implies `len - already_absorbed == i`.
        let offset = builder.sub_extension(local_values.len, already_absorbed_bytes);
        for (i, &is_final_len) in local_values.is_final_input_len.iter().enumerate() {
            let index = builder.constant_extension(F::from_canonical_usize(i).into());
            let entry_match = builder.sub_extension(offset, index);

            let constraint = builder.mul_extension(is_final_len, entry_match);
            yield_constr.constraint(builder, constraint);
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
    use itertools::Itertools;
    use keccak_hash::keccak;
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::types::PrimeField64;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    use crate::keccak_sponge::columns::KeccakSpongeColumnsView;
    use crate::keccak_sponge::keccak_sponge_stark::{KeccakSpongeOp, KeccakSpongeStark};
    use crate::memory::segments::Segment;
    use crate::stark_testing::{test_stark_circuit_constraints, test_stark_low_degree};
    use crate::witness::memory::MemoryAddress;

    #[test]
    fn test_stark_degree() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = KeccakSpongeStark<F, D>;

        let stark = S::default();
        test_stark_low_degree(stark)
    }

    #[test]
    fn test_stark_circuit() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = KeccakSpongeStark<F, D>;

        let stark = S::default();
        test_stark_circuit_constraints::<F, C, S, D>(stark)
    }

    #[test]
    fn test_generation() -> Result<()> {
        const D: usize = 2;
        type F = GoldilocksField;
        type S = KeccakSpongeStark<F, D>;

        let input = vec![1, 2, 3];
        let expected_output = keccak(&input);

        let op = KeccakSpongeOp {
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
        let last_row: &KeccakSpongeColumnsView<F> = rows.last().unwrap().borrow();
        let output = last_row
            .updated_digest_state_bytes
            .iter()
            .map(|x| x.to_canonical_u64() as u8)
            .collect_vec();

        assert_eq!(output, expected_output.0);
        Ok(())
    }
}
