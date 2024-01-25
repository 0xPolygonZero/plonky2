use std::borrow::Borrow;
use std::iter::{self, once, repeat};
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
use plonky2::util::transpose;
use plonky2_util::ceil_div_usize;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::kernel::keccak_util::keccakf_u32s;
use crate::cross_table_lookup::{Column, Filter};
use crate::evaluation_frame::{StarkEvaluationFrame, StarkFrame};
use crate::keccak_sponge::columns::*;
use crate::lookup::Lookup;
use crate::stark::Stark;
use crate::witness::memory::MemoryAddress;

/// Strict upper bound for the individual bytes range-check.
const BYTE_RANGE_MAX: usize = 256;

/// Creates the vector of `Columns` corresponding to:
/// - the address in memory of the inputs,
/// - the length of the inputs,
/// - the timestamp at which the inputs are read from memory,
/// - the output limbs of the Keccak sponge.
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

    // The length of the inputs is `already_absorbed_bytes + is_final_input_len`.
    let len_col = Column::linear_combination(
        iter::once((cols.already_absorbed_bytes, F::ONE)).chain(
            cols.is_final_input_len
                .iter()
                .enumerate()
                .map(|(i, &elt)| (elt, F::from_canonical_usize(i))),
        ),
    );

    let mut res: Vec<Column<F>> =
        Column::singles([cols.context, cols.segment, cols.virt]).collect();
    res.push(len_col);
    res.push(Column::single(cols.timestamp));
    res.extend(outputs);

    res
}

/// Creates the vector of `Columns` corresponding to the inputs of the Keccak sponge.
/// This is used to check that the inputs of the sponge correspond to the inputs
/// given by `KeccakStark`.
pub(crate) fn ctl_looking_keccak_inputs<F: Field>() -> Vec<Column<F>> {
    let cols = KECCAK_SPONGE_COL_MAP;
    let mut res: Vec<_> = Column::singles(
        [
            cols.xored_rate_u32s.as_slice(),
            &cols.original_capacity_u32s,
        ]
        .concat(),
    )
    .collect();
    res.push(Column::single(cols.timestamp));

    res
}

/// Creates the vector of `Columns` corresponding to the outputs of the Keccak sponge.
/// This is used to check that the outputs of the sponge correspond to the outputs
/// given by `KeccakStark`.
pub(crate) fn ctl_looking_keccak_outputs<F: Field>() -> Vec<Column<F>> {
    let cols = KECCAK_SPONGE_COL_MAP;

    // We recover the 32-bit digest limbs from their corresponding bytes,
    // and then append them to the rest of the updated state limbs.
    let digest_u32s = cols.updated_digest_state_bytes.chunks_exact(4).map(|c| {
        Column::linear_combination(
            c.iter()
                .enumerate()
                .map(|(i, &b)| (b, F::from_canonical_usize(1 << (8 * i)))),
        )
    });

    let mut res: Vec<_> = digest_u32s.collect();

    res.extend(Column::singles(&cols.partial_updated_state_u32s));
    res.push(Column::single(cols.timestamp));

    res
}

/// Creates the vector of `Columns` corresponding to the address and value of the byte being read from memory.
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

/// Returns the number of `KeccakSponge` tables looking into the `LogicStark`.
pub(crate) const fn num_logic_ctls() -> usize {
    const U8S_PER_CTL: usize = 32;
    ceil_div_usize(KECCAK_RATE_BYTES, U8S_PER_CTL)
}

/// Creates the vector of `Columns` required to perform the `i`th logic CTL.
/// It is comprised of the ÌS_XOR` flag, the two inputs and the output
/// of the XOR operation.
/// Since we need to do 136 byte XORs, and the logic CTL can
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

/// CTL filter for the final block rows of the `KeccakSponge` table.
pub(crate) fn ctl_looked_filter<F: Field>() -> Filter<F> {
    // The CPU table is only interested in our final-block rows, since those contain the final
    // sponge output.
    Filter::new_simple(Column::sum(KECCAK_SPONGE_COL_MAP.is_final_input_len))
}

/// CTL filter for reading the `i`th byte of input from memory.
pub(crate) fn ctl_looking_memory_filter<F: Field>(i: usize) -> Filter<F> {
    // We perform the `i`th read if either
    // - this is a full input block, or
    // - this is a final block of length `i` or greater
    let cols = KECCAK_SPONGE_COL_MAP;
    if i == KECCAK_RATE_BYTES - 1 {
        Filter::new_simple(Column::single(cols.is_full_input_block))
    } else {
        Filter::new_simple(Column::sum(
            once(&cols.is_full_input_block).chain(&cols.is_final_input_len[i + 1..]),
        ))
    }
}

/// CTL filter for looking at XORs in the logic table.
pub(crate) fn ctl_looking_logic_filter<F: Field>() -> Filter<F> {
    let cols = KECCAK_SPONGE_COL_MAP;
    Filter::new_simple(Column::sum(
        once(&cols.is_full_input_block).chain(&cols.is_final_input_len),
    ))
}

/// CTL filter for looking at the input and output in the Keccak table.
pub(crate) fn ctl_looking_keccak_filter<F: Field>() -> Filter<F> {
    let cols = KECCAK_SPONGE_COL_MAP;
    Filter::new_simple(Column::sum(
        once(&cols.is_full_input_block).chain(&cols.is_final_input_len),
    ))
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

/// Structure representing the `KeccakSponge` STARK, which carries out the sponge permutation.
#[derive(Copy, Clone, Default)]
pub(crate) struct KeccakSpongeStark<F, const D: usize> {
    f: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> KeccakSpongeStark<F, D> {
    /// Generates the trace polynomial values for the `KeccakSponge`STARK.
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

        let trace_row_vecs: Vec<_> = trace_rows.into_iter().map(|row| row.to_vec()).collect();

        let mut trace_cols = transpose(&trace_row_vecs);
        self.generate_range_checks(&mut trace_cols);

        trace_cols.into_iter().map(PolynomialValues::new).collect()
    }

    /// Generates the trace rows given the vector of `KeccakSponge` operations.
    /// The trace is padded to a power of two with all-zero rows.
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
        // Generate active rows.
        for op in operations {
            rows.extend(self.generate_rows_for_op(op));
        }
        // Pad the trace.
        let padded_rows = rows.len().max(min_rows).next_power_of_two();
        for _ in rows.len()..padded_rows {
            rows.push(self.generate_padding_row());
        }
        rows
    }

    /// Generates the rows associated to a given operation:
    /// Performs a Keccak sponge permutation and fills the STARK's rows accordingly.
    /// The number of rows is the number of input chunks of size `KECCAK_RATE_BYTES`.
    fn generate_rows_for_op(&self, op: KeccakSpongeOp) -> Vec<[F; NUM_KECCAK_SPONGE_COLUMNS]> {
        let mut rows = Vec::with_capacity(op.input.len() / KECCAK_RATE_BYTES + 1);

        let mut sponge_state = [0u32; KECCAK_WIDTH_U32S];

        let mut input_blocks = op.input.chunks_exact(KECCAK_RATE_BYTES);
        let mut already_absorbed_bytes = 0;
        for block in input_blocks.by_ref() {
            // We compute the updated state of the sponge.
            let row = self.generate_full_input_row(
                &op,
                already_absorbed_bytes,
                sponge_state,
                block.try_into().unwrap(),
            );

            // We update the state limbs for the next block absorption.
            // The first `KECCAK_DIGEST_U32s` limbs are stored as bytes after the computation,
            // so we recompute the corresponding `u32` and update the first state limbs.
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

            // The rest of the bytes are already stored in the expected form, so we can directly
            // update the state with the stored values.
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

    /// Generates a row where all bytes are input bytes, not padding bytes.
    /// This includes updating the state sponge with a single absorption.
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

    /// Generates a row containing the last input bytes.
    /// On top of computing one absorption and padding the input,
    /// we indicate the last non-padding input byte by setting
    /// `row.is_final_input_len[final_inputs.len()]` to 1.
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
    /// Given a state S = R || C and a block input B,
    /// - R is updated with R XOR B,
    /// - S is replaced by keccakf_u32s(S).
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

    /// Expects input in *column*-major layout
    fn generate_range_checks(&self, cols: &mut [Vec<F>]) {
        debug_assert!(cols.len() == NUM_KECCAK_SPONGE_COLUMNS);

        let n_rows = cols[0].len();
        debug_assert!(cols.iter().all(|col| col.len() == n_rows));

        for i in 0..BYTE_RANGE_MAX {
            cols[RANGE_COUNTER][i] = F::from_canonical_usize(i);
        }
        for i in BYTE_RANGE_MAX..n_rows {
            cols[RANGE_COUNTER][i] = F::from_canonical_usize(BYTE_RANGE_MAX - 1);
        }

        // For each column c in cols, generate the range-check
        // permutations and put them in the corresponding range-check
        // columns rc_c and rc_c+1.
        for col in 0..KECCAK_RATE_BYTES {
            let c = get_single_block_bytes_value(col);
            for i in 0..n_rows {
                let x = cols[c][i].to_canonical_u64() as usize;
                assert!(
                    x < BYTE_RANGE_MAX,
                    "column value {} exceeds the max range value {}",
                    x,
                    BYTE_RANGE_MAX
                );
                cols[RC_FREQUENCIES][x] += F::ONE;
            }
        }
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for KeccakSpongeStark<F, D> {
    type EvaluationFrame<FE, P, const D2: usize> = StarkFrame<P, NUM_KECCAK_SPONGE_COLUMNS>
    where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>;

    type EvaluationFrameTarget = StarkFrame<ExtensionTarget<D>, NUM_KECCAK_SPONGE_COLUMNS>;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: &Self::EvaluationFrame<FE, P, D2>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        let local_values: &[P; NUM_KECCAK_SPONGE_COLUMNS] =
            vars.get_local_values().try_into().unwrap();
        let local_values: &KeccakSpongeColumnsView<P> = local_values.borrow();
        let next_values: &[P; NUM_KECCAK_SPONGE_COLUMNS] =
            vars.get_next_values().try_into().unwrap();
        let next_values: &KeccakSpongeColumnsView<P> = next_values.borrow();

        // Check the range column: First value must be 0, last row
        // must be 255, and intermediate rows must increment by 0
        // or 1.
        let rc1 = local_values.range_counter;
        let rc2 = next_values.range_counter;
        yield_constr.constraint_first_row(rc1);
        let incr = rc2 - rc1;
        yield_constr.constraint_transition(incr * incr - incr);
        let range_max = P::Scalar::from_canonical_u64((BYTE_RANGE_MAX - 1) as u64);
        yield_constr.constraint_last_row(rc1 - range_max);

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
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: &Self::EvaluationFrameTarget,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let local_values: &[ExtensionTarget<D>; NUM_KECCAK_SPONGE_COLUMNS] =
            vars.get_local_values().try_into().unwrap();
        let local_values: &KeccakSpongeColumnsView<ExtensionTarget<D>> = local_values.borrow();
        let next_values: &[ExtensionTarget<D>; NUM_KECCAK_SPONGE_COLUMNS] =
            vars.get_next_values().try_into().unwrap();
        let next_values: &KeccakSpongeColumnsView<ExtensionTarget<D>> = next_values.borrow();

        let one = builder.one_extension();

        // Check the range column: First value must be 0, last row
        // must be 255, and intermediate rows must increment by 0
        // or 1.
        let rc1 = local_values.range_counter;
        let rc2 = next_values.range_counter;
        yield_constr.constraint_first_row(builder, rc1);
        let incr = builder.sub_extension(rc2, rc1);
        let t = builder.mul_sub_extension(incr, incr, incr);
        yield_constr.constraint_transition(builder, t);
        let range_max =
            builder.constant_extension(F::Extension::from_canonical_usize(BYTE_RANGE_MAX - 1));
        let t = builder.sub_extension(rc1, range_max);
        yield_constr.constraint_last_row(builder, t);

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
    }

    fn constraint_degree(&self) -> usize {
        3
    }

    fn lookups(&self) -> Vec<Lookup<F>> {
        vec![Lookup {
            columns: Column::singles(get_block_bytes_range()).collect(),
            table_column: Column::single(RANGE_COUNTER),
            frequencies_column: Column::single(RC_FREQUENCIES),
            filter_columns: vec![None; KECCAK_RATE_BYTES],
        }]
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
            base_address: MemoryAddress::new(0, Segment::Code, 0),
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
