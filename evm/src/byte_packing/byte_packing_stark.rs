//! This crate enforces the correctness of reading and writing sequences
//! of bytes in Big-Endian ordering from and to the memory.
//!
//! The trace layout consists in N consecutive rows for an `N` byte sequence,
//! with the byte values being cumulatively written to the trace as they are
//! being processed.
//!
//! At row `i` of such a group (starting from 0), the `i`-th byte flag will be activated
//! (to indicate which byte we are going to be processing), but all bytes with index
//! 0 to `i` may have non-zero values, as they have already been processed.
//!
//! The length of a sequence is stored within each group of rows corresponding to that
//! sequence in a dedicated `SEQUENCE_LEN` column. At any row `i`, the remaining length
//! of the sequence being processed is retrieved from that column and the active byte flag
//! as:
//!
//!    remaining_length = sequence_length - \sum_{i=0}^31 b[i] * i
//!
//! where b[i] is the `i`-th byte flag.
//!
//! Because of the discrepancy in endianness between the different tables, the byte sequences
//! are actually written in the trace in reverse order from the order they are provided.
//! As such, the memory virtual address for a group of rows corresponding to a sequence starts
//! with the final virtual address, corresponding to the final byte being read/written, and
//! is being decremented at each step.
//!
//! Note that, when writing a sequence of bytes to memory, both the `U256` value and the
//! corresponding sequence length are being read from the stack. Because of the endianness
//! discrepancy mentioned above, we first convert the value to a byte sequence in Little-Endian,
//! then resize the sequence to prune unneeded zeros before reverting the sequence order.
//! This means that the higher-order bytes will be thrown away during the process, if the value
//! is greater than 256^length, and as a result a different value will be stored in memory.

use std::marker::PhantomData;

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

use super::NUM_BYTES;
use crate::byte_packing::columns::{
    index_bytes, value_bytes, ADDR_CONTEXT, ADDR_SEGMENT, ADDR_VIRTUAL, BYTE_INDICES_COLS, IS_READ,
    NUM_COLUMNS, RANGE_COUNTER, RC_COLS, SEQUENCE_END, TIMESTAMP,
};
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cross_table_lookup::Column;
use crate::lookup::{eval_lookups, eval_lookups_circuit, permuted_cols};
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};
use crate::witness::memory::MemoryAddress;

/// Strict upper bound for the individual bytes range-check.
const BYTE_RANGE_MAX: usize = 1usize << 8;

pub(crate) fn ctl_looked_data<F: Field>() -> Vec<Column<F>> {
    // Reconstruct the u32 limbs composing the final `U256` word
    // being read/written from the underlying byte values. For each,
    // we pack 4 consecutive bytes and shift them accordingly to
    // obtain the corresponding limb.
    let outputs: Vec<Column<F>> = (0..8)
        .map(|i| {
            let range = (value_bytes(i * 4)..value_bytes(i * 4) + 4).collect_vec();
            Column::linear_combination(
                range
                    .iter()
                    .enumerate()
                    .map(|(j, &c)| (c, F::from_canonical_u64(1 << (8 * j)))),
            )
        })
        .collect();

    // This will correspond to the actual sequence length when the `SEQUENCE_END` flag is on.
    let sequence_len: Column<F> = Column::linear_combination(
        (0..NUM_BYTES).map(|i| (index_bytes(i), F::from_canonical_usize(i + 1))),
    );

    Column::singles([ADDR_CONTEXT, ADDR_SEGMENT, ADDR_VIRTUAL])
        .chain([sequence_len])
        .chain(Column::singles(&[TIMESTAMP]))
        .chain(outputs)
        .collect()
}

pub fn ctl_looked_filter<F: Field>() -> Column<F> {
    // The CPU table is only interested in our sequence end rows,
    // since those contain the final limbs of our packed int.
    Column::single(SEQUENCE_END)
}

pub(crate) fn ctl_looking_memory<F: Field>(i: usize) -> Vec<Column<F>> {
    let mut res =
        Column::singles([IS_READ, ADDR_CONTEXT, ADDR_SEGMENT, ADDR_VIRTUAL]).collect_vec();

    // The i'th input byte being read/written.
    res.push(Column::single(value_bytes(i)));

    // Since we're reading a single byte, the higher limbs must be zero.
    res.extend((1..8).map(|_| Column::zero()));

    res.push(Column::single(TIMESTAMP));

    res
}

/// CTL filter for reading/writing the `i`th byte of the byte sequence from/to memory.
pub(crate) fn ctl_looking_memory_filter<F: Field>(i: usize) -> Column<F> {
    Column::single(index_bytes(i))
}

/// Information about a byte packing operation needed for witness generation.
#[derive(Clone, Debug)]
pub(crate) struct BytePackingOp {
    /// Whether this is a read (packing) or write (unpacking) operation.
    pub(crate) is_read: bool,

    /// The base address at which inputs are read/written.
    pub(crate) base_address: MemoryAddress,

    /// The timestamp at which inputs are read/written.
    pub(crate) timestamp: usize,

    /// The byte sequence that was read/written.
    /// Its length is required to be at most 32.
    pub(crate) bytes: Vec<u8>,
}

#[derive(Copy, Clone, Default)]
pub struct BytePackingStark<F, const D: usize> {
    pub(crate) f: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> BytePackingStark<F, D> {
    pub(crate) fn generate_trace(
        &self,
        ops: Vec<BytePackingOp>,
        min_rows: usize,
        timing: &mut TimingTree,
    ) -> Vec<PolynomialValues<F>> {
        // Generate most of the trace in row-major form.
        let trace_rows = timed!(
            timing,
            "generate trace rows",
            self.generate_trace_rows(ops, min_rows)
        );
        let trace_row_vecs: Vec<_> = trace_rows.into_iter().map(|row| row.to_vec()).collect();

        let mut trace_cols = transpose(&trace_row_vecs);
        self.generate_range_checks(&mut trace_cols);

        trace_cols.into_iter().map(PolynomialValues::new).collect()
    }

    fn generate_trace_rows(
        &self,
        ops: Vec<BytePackingOp>,
        min_rows: usize,
    ) -> Vec<[F; NUM_COLUMNS]> {
        let base_len: usize = ops.iter().map(|op| op.bytes.len()).sum();
        let num_rows = core::cmp::max(base_len.max(BYTE_RANGE_MAX), min_rows).next_power_of_two();
        let mut rows = Vec::with_capacity(num_rows);

        for op in ops {
            rows.extend(self.generate_rows_for_op(op));
        }

        for _ in rows.len()..num_rows {
            rows.push(self.generate_padding_row());
        }

        rows
    }

    fn generate_rows_for_op(&self, op: BytePackingOp) -> Vec<[F; NUM_COLUMNS]> {
        let BytePackingOp {
            is_read,
            base_address,
            timestamp,
            bytes,
        } = op;

        let MemoryAddress {
            context,
            segment,
            virt,
        } = base_address;

        let mut rows = Vec::with_capacity(bytes.len());
        let mut row = [F::ZERO; NUM_COLUMNS];
        row[IS_READ] = F::from_bool(is_read);

        row[ADDR_CONTEXT] = F::from_canonical_usize(context);
        row[ADDR_SEGMENT] = F::from_canonical_usize(segment);
        // Because of the endianness, we start by the final virtual address value
        // and decrement it at each step. Similarly, we process the byte sequence
        // in reverse order.
        row[ADDR_VIRTUAL] = F::from_canonical_usize(virt + bytes.len() - 1);

        row[TIMESTAMP] = F::from_canonical_usize(timestamp);

        for (i, &byte) in bytes.iter().rev().enumerate() {
            if i == bytes.len() - 1 {
                row[SEQUENCE_END] = F::ONE;
            }
            row[value_bytes(i)] = F::from_canonical_u8(byte);
            row[index_bytes(i)] = F::ONE;

            rows.push(row.into());
            row[index_bytes(i)] = F::ZERO;
            row[ADDR_VIRTUAL] -= F::ONE;
        }

        rows
    }

    fn generate_padding_row(&self) -> [F; NUM_COLUMNS] {
        [F::ZERO; NUM_COLUMNS]
    }

    /// Expects input in *column*-major layout
    fn generate_range_checks(&self, cols: &mut Vec<Vec<F>>) {
        debug_assert!(cols.len() == NUM_COLUMNS);

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
        for (i, rc_c) in (0..NUM_BYTES).zip(RC_COLS.step_by(2)) {
            let c = value_bytes(i);
            let (col_perm, table_perm) = permuted_cols(&cols[c], &cols[RANGE_COUNTER]);
            cols[rc_c].copy_from_slice(&col_perm);
            cols[rc_c + 1].copy_from_slice(&table_perm);
        }
    }

    /// There is only one `i` for which `vars.local_values[index_bytes(i)]` is non-zero,
    /// and `i+1` is the current position:
    fn get_active_position<FE, P, const D2: usize>(&self, row: &[P; NUM_COLUMNS]) -> P
    where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        (0..NUM_BYTES)
            .map(|i| row[index_bytes(i)] * P::Scalar::from_canonical_usize(i + 1))
            .sum()
    }

    /// Recursive version of `get_active_position`.
    fn get_active_position_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        row: &[ExtensionTarget<D>; NUM_COLUMNS],
    ) -> ExtensionTarget<D> {
        let mut current_position = row[index_bytes(0)];

        for i in 1..NUM_BYTES {
            current_position = builder.mul_const_add_extension(
                F::from_canonical_usize(i + 1),
                row[index_bytes(i)],
                current_position,
            );
        }

        current_position
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for BytePackingStark<F, D> {
    const COLUMNS: usize = NUM_COLUMNS;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        // Range check all the columns
        for col in RC_COLS.step_by(2) {
            eval_lookups(vars, yield_constr, col, col + 1);
        }

        let one = P::ONES;

        // We filter active columns by summing all the byte indices.
        // Constraining each of them to be boolean is done later on below.
        let current_filter = vars.local_values[BYTE_INDICES_COLS]
            .iter()
            .copied()
            .sum::<P>();
        yield_constr.constraint(current_filter * (current_filter - one));

        // The filter column must start by one.
        yield_constr.constraint_first_row(current_filter - one);

        // The is_read flag must be boolean.
        let current_is_read = vars.local_values[IS_READ];
        yield_constr.constraint(current_is_read * (current_is_read - one));

        // Each byte index must be boolean.
        for i in 0..NUM_BYTES {
            let idx_i = vars.local_values[index_bytes(i)];
            yield_constr.constraint(idx_i * (idx_i - one));
        }

        // The sequence start flag column must start by one.
        let current_sequence_start = vars.local_values[index_bytes(0)];
        yield_constr.constraint_first_row(current_sequence_start - one);

        // The sequence end flag must be boolean
        let current_sequence_end = vars.local_values[SEQUENCE_END];
        yield_constr.constraint(current_sequence_end * (current_sequence_end - one));

        // If filter is off, all flags and byte indices must be off.
        let byte_indices = vars.local_values[BYTE_INDICES_COLS]
            .iter()
            .copied()
            .sum::<P>();
        yield_constr.constraint(
            (current_filter - one) * (current_is_read + current_sequence_end + byte_indices),
        );

        // Only padding rows have their filter turned off.
        let next_filter = vars.next_values[BYTE_INDICES_COLS]
            .iter()
            .copied()
            .sum::<P>();
        yield_constr.constraint_transition(next_filter * (next_filter - current_filter));

        // Unless the current sequence end flag is activated, the is_read filter must remain unchanged.
        let next_is_read = vars.next_values[IS_READ];
        yield_constr
            .constraint_transition((current_sequence_end - one) * (next_is_read - current_is_read));

        // If the sequence end flag is activated, the next row must be a new sequence or filter must be off.
        let next_sequence_start = vars.next_values[index_bytes(0)];
        yield_constr.constraint_transition(
            current_sequence_end * next_filter * (next_sequence_start - one),
        );

        // The active position in a byte sequence must increase by one on every row
        // or be one on the next row (i.e. at the start of a new sequence).
        let current_position = self.get_active_position(vars.local_values);
        let next_position = self.get_active_position(vars.next_values);
        yield_constr.constraint_transition(
            next_filter * (next_position - one) * (next_position - current_position - one),
        );

        // The last row must be the end of a sequence or a padding row.
        yield_constr.constraint_last_row(current_filter * (current_sequence_end - one));

        // If the next position is one in an active row, the current end flag must be one.
        yield_constr
            .constraint_transition(next_filter * current_sequence_end * (next_position - one));

        // The context, segment and timestamp fields must remain unchanged throughout a byte sequence.
        // The virtual address must decrement by one at each step of a sequence.
        let current_context = vars.local_values[ADDR_CONTEXT];
        let next_context = vars.next_values[ADDR_CONTEXT];
        let current_segment = vars.local_values[ADDR_SEGMENT];
        let next_segment = vars.next_values[ADDR_SEGMENT];
        let current_virtual = vars.local_values[ADDR_VIRTUAL];
        let next_virtual = vars.next_values[ADDR_VIRTUAL];
        let current_timestamp = vars.local_values[TIMESTAMP];
        let next_timestamp = vars.next_values[TIMESTAMP];
        yield_constr.constraint_transition(
            next_filter * (next_sequence_start - one) * (next_context - current_context),
        );
        yield_constr.constraint_transition(
            next_filter * (next_sequence_start - one) * (next_segment - current_segment),
        );
        yield_constr.constraint_transition(
            next_filter * (next_sequence_start - one) * (next_timestamp - current_timestamp),
        );
        yield_constr.constraint_transition(
            next_filter * (next_sequence_start - one) * (current_virtual - next_virtual - one),
        );

        // If not at the end of a sequence, each next byte must equal the current one
        // when reading through the sequence, or the next byte index must be one.
        for i in 0..NUM_BYTES {
            let current_byte = vars.local_values[value_bytes(i)];
            let next_byte = vars.next_values[value_bytes(i)];
            let next_byte_index = vars.next_values[index_bytes(i)];
            yield_constr.constraint_transition(
                (current_sequence_end - one) * (next_byte_index - one) * (next_byte - current_byte),
            );
        }
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        // Range check all the columns
        for col in RC_COLS.step_by(2) {
            eval_lookups_circuit(builder, vars, yield_constr, col, col + 1);
        }

        // We filter active columns by summing all the byte indices.
        // Constraining each of them to be boolean is done later on below.
        let current_filter = builder.add_many_extension(&vars.local_values[BYTE_INDICES_COLS]);
        let constraint = builder.mul_sub_extension(current_filter, current_filter, current_filter);
        yield_constr.constraint(builder, constraint);

        // The filter column must start by one.
        let constraint = builder.add_const_extension(current_filter, F::NEG_ONE);
        yield_constr.constraint_first_row(builder, constraint);

        // The is_read flag must be boolean.
        let current_is_read = vars.local_values[IS_READ];
        let constraint =
            builder.mul_sub_extension(current_is_read, current_is_read, current_is_read);
        yield_constr.constraint(builder, constraint);

        // Each byte index must be boolean.
        for i in 0..NUM_BYTES {
            let idx_i = vars.local_values[index_bytes(i)];
            let constraint = builder.mul_sub_extension(idx_i, idx_i, idx_i);
            yield_constr.constraint(builder, constraint);
        }

        // The sequence start flag column must start by one.
        let current_sequence_start = vars.local_values[index_bytes(0)];
        let constraint = builder.add_const_extension(current_sequence_start, F::NEG_ONE);
        yield_constr.constraint_first_row(builder, constraint);

        // The sequence end flag must be boolean
        let current_sequence_end = vars.local_values[SEQUENCE_END];
        let constraint = builder.mul_sub_extension(
            current_sequence_end,
            current_sequence_end,
            current_sequence_end,
        );
        yield_constr.constraint(builder, constraint);

        // If filter is off, all flags and byte indices must be off.
        let byte_indices = builder.add_many_extension(&vars.local_values[BYTE_INDICES_COLS]);
        let constraint = builder.add_extension(current_sequence_end, byte_indices);
        let constraint = builder.add_extension(constraint, current_is_read);
        let constraint = builder.mul_sub_extension(constraint, current_filter, constraint);
        yield_constr.constraint(builder, constraint);

        // Only padding rows have their filter turned off.
        let next_filter = builder.add_many_extension(&vars.next_values[BYTE_INDICES_COLS]);
        let constraint = builder.sub_extension(next_filter, current_filter);
        let constraint = builder.mul_extension(next_filter, constraint);
        yield_constr.constraint_transition(builder, constraint);

        // Unless the current sequence end flag is activated, the is_read filter must remain unchanged.
        let next_is_read = vars.next_values[IS_READ];
        let diff_is_read = builder.sub_extension(next_is_read, current_is_read);
        let constraint =
            builder.mul_sub_extension(diff_is_read, current_sequence_end, diff_is_read);
        yield_constr.constraint_transition(builder, constraint);

        // If the sequence end flag is activated, the next row must be a new sequence or filter must be off.
        let next_sequence_start = vars.next_values[index_bytes(0)];
        let constraint = builder.mul_sub_extension(
            current_sequence_end,
            next_sequence_start,
            current_sequence_end,
        );
        let constraint = builder.mul_extension(next_filter, constraint);
        yield_constr.constraint_transition(builder, constraint);

        // The active position in a byte sequence must increase by one on every row
        // or be one on the next row (i.e. at the start of a new sequence).
        let current_position = self.get_active_position_circuit(builder, vars.local_values);
        let next_position = self.get_active_position_circuit(builder, vars.next_values);

        let position_diff = builder.sub_extension(next_position, current_position);
        let is_new_or_inactive = builder.mul_sub_extension(next_filter, next_position, next_filter);
        let constraint =
            builder.mul_sub_extension(is_new_or_inactive, position_diff, is_new_or_inactive);
        yield_constr.constraint_transition(builder, constraint);

        // The last row must be the end of a sequence or a padding row.
        let constraint =
            builder.mul_sub_extension(current_filter, current_sequence_end, current_filter);
        yield_constr.constraint_last_row(builder, constraint);

        // If the next position is one in an active row, the current end flag must be one.
        let constraint = builder.mul_extension(next_filter, current_sequence_end);
        let constraint = builder.mul_sub_extension(constraint, next_position, constraint);
        yield_constr.constraint_transition(builder, constraint);

        // The context, segment and timestamp fields must remain unchanged throughout a byte sequence.
        // The virtual address must decrement by one at each step of a sequence.
        let current_context = vars.local_values[ADDR_CONTEXT];
        let next_context = vars.next_values[ADDR_CONTEXT];
        let current_segment = vars.local_values[ADDR_SEGMENT];
        let next_segment = vars.next_values[ADDR_SEGMENT];
        let current_virtual = vars.local_values[ADDR_VIRTUAL];
        let next_virtual = vars.next_values[ADDR_VIRTUAL];
        let current_timestamp = vars.local_values[TIMESTAMP];
        let next_timestamp = vars.next_values[TIMESTAMP];
        let addr_filter = builder.mul_sub_extension(next_filter, next_sequence_start, next_filter);
        {
            let constraint = builder.sub_extension(next_context, current_context);
            let constraint = builder.mul_extension(addr_filter, constraint);
            yield_constr.constraint_transition(builder, constraint);
        }
        {
            let constraint = builder.sub_extension(next_segment, current_segment);
            let constraint = builder.mul_extension(addr_filter, constraint);
            yield_constr.constraint_transition(builder, constraint);
        }
        {
            let constraint = builder.sub_extension(next_timestamp, current_timestamp);
            let constraint = builder.mul_extension(addr_filter, constraint);
            yield_constr.constraint_transition(builder, constraint);
        }
        {
            let constraint = builder.sub_extension(current_virtual, next_virtual);
            let constraint = builder.mul_sub_extension(addr_filter, constraint, addr_filter);
            yield_constr.constraint_transition(builder, constraint);
        }

        // If not at the end of a sequence, each next byte must equal the current one
        // when reading through the sequence, or the next byte index must be one.
        for i in 0..NUM_BYTES {
            let current_byte = vars.local_values[value_bytes(i)];
            let next_byte = vars.next_values[value_bytes(i)];
            let next_byte_index = vars.next_values[index_bytes(i)];
            let byte_diff = builder.sub_extension(next_byte, current_byte);
            let constraint = builder.mul_sub_extension(byte_diff, next_byte_index, byte_diff);
            let constraint =
                builder.mul_sub_extension(constraint, current_sequence_end, constraint);
            yield_constr.constraint_transition(builder, constraint);
        }
    }

    fn constraint_degree(&self) -> usize {
        3
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use anyhow::Result;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    use crate::byte_packing::byte_packing_stark::BytePackingStark;
    use crate::stark_testing::{test_stark_circuit_constraints, test_stark_low_degree};

    #[test]
    fn test_stark_degree() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = BytePackingStark<F, D>;

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
        type S = BytePackingStark<F, D>;

        let stark = S {
            f: Default::default(),
        };
        test_stark_circuit_constraints::<F, C, S, D>(stark)
    }
}
