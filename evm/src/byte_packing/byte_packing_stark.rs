use std::marker::PhantomData;

use itertools::Itertools;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::timed;
use plonky2::util::timing::TimingTree;

use super::columns::{
    index_bytes, ADDR_CONTEXT, ADDR_SEGMENT, ADDR_VIRTUAL, IS_READ, SEQUENCE_END, TIMESTAMP,
};
use super::NUM_BYTES;
use crate::byte_packing::columns::{value_bytes, FILTER, NUM_COLUMNS, REMAINING_LEN, SEQUENCE_LEN};
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cross_table_lookup::Column;
use crate::stark::Stark;
use crate::util::trace_rows_to_poly_values;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};
use crate::witness::memory::MemoryAddress;

pub(crate) fn ctl_looked_data<F: Field>() -> Vec<Column<F>> {
    let outputs: Vec<Column<F>> = (0..8)
        .map(|i| {
            let range = (value_bytes(i * 4)..value_bytes((i + 1) * 4)).collect_vec();
            Column::linear_combination(
                range
                    .iter()
                    .enumerate()
                    .map(|(j, &c)| (c, F::from_canonical_u64(1 << 8 * j))),
            )
        })
        .collect();

    Column::singles([
        ADDR_CONTEXT,
        ADDR_SEGMENT,
        ADDR_VIRTUAL,
        SEQUENCE_LEN,
        TIMESTAMP,
    ])
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

    // The i'th input byte being read.
    res.push(Column::single(value_bytes(i)));

    // Since we're reading a single byte, the higher limbs must be zero.
    res.extend((1..8).map(|_| Column::zero()));

    res.push(Column::single(TIMESTAMP));

    res
}

/// CTL filter for reading the `i`th byte of the byte sequence from memory.
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
    /// Its length is expected to be at most 32.
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

        let trace_polys = timed!(
            timing,
            "convert to PolynomialValues",
            trace_rows_to_poly_values(trace_rows)
        );

        trace_polys
    }

    fn generate_trace_rows(
        &self,
        ops: Vec<BytePackingOp>,
        min_rows: usize,
    ) -> Vec<[F; NUM_COLUMNS]> {
        let base_len: usize = ops.iter().map(|op| op.bytes.len()).sum();
        let mut rows = Vec::with_capacity(base_len.max(min_rows).next_power_of_two());

        for op in ops {
            rows.extend(self.generate_rows_for_op(op));
        }

        let padded_rows = rows.len().max(min_rows).next_power_of_two();
        for _ in rows.len()..padded_rows {
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
        row[FILTER] = F::ONE;
        row[IS_READ] = F::from_bool(is_read);

        row[ADDR_CONTEXT] = F::from_canonical_usize(context);
        row[ADDR_SEGMENT] = F::from_canonical_usize(segment);
        // Because of the endianness, we start by the final virtual address value
        // and decrement it at each step.
        row[ADDR_VIRTUAL] = F::from_canonical_usize(virt + bytes.len() - 1);

        row[TIMESTAMP] = F::from_canonical_usize(timestamp);
        row[SEQUENCE_LEN] = F::from_canonical_usize(bytes.len());

        for (i, &byte) in bytes.iter().rev().enumerate() {
            row[REMAINING_LEN] = F::from_canonical_usize(bytes.len() - 1 - i);
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
        let one = P::ONES;

        // The filter must be boolean.
        let filter = vars.local_values[FILTER];
        yield_constr.constraint(filter * (filter - one));

        // The filter column must start by one.
        yield_constr.constraint_first_row(filter - one);

        // The is_read flag must be boolean.
        let is_read = vars.local_values[IS_READ];
        yield_constr.constraint(is_read * (is_read - one));

        // If filter is off, is_read must be off.
        let is_read = vars.local_values[IS_READ];
        yield_constr.constraint((filter - one) * is_read);

        // Only padding rows have their filter turned off.
        let next_filter = vars.next_values[FILTER];
        yield_constr.constraint_transition(next_filter * (next_filter - filter));

        // The sequence start flag column must start by one.
        let sequence_start = vars.local_values[index_bytes(0)];
        yield_constr.constraint_first_row(sequence_start - one);

        // The sequence end flag must be boolean
        let sequence_end = vars.local_values[SEQUENCE_END];
        yield_constr.constraint(sequence_end * (sequence_end - one));

        // If the sequence end flag is activated, the next row must be a new sequence or filter must be off.
        let next_sequence_start = vars.next_values[index_bytes(0)];
        yield_constr
            .constraint_transition(sequence_end * next_filter * (next_sequence_start - one));

        // Each byte index must be boolean.
        for i in 0..NUM_BYTES {
            let idx_i = vars.local_values[index_bytes(i)];
            yield_constr.constraint(idx_i * (idx_i - one));
        }

        // There must be only one byte index set to 1 per active row.
        let sum_indices = vars.local_values[index_bytes(0)..index_bytes(0) + NUM_BYTES]
            .iter()
            .copied()
            .sum::<P>();
        yield_constr.constraint(filter * (sum_indices - P::ONES));

        // The remaining length of a byte sequence must decrease by one or be zero.
        let current_remaining_length = vars.local_values[REMAINING_LEN];
        let next_remaining_length = vars.next_values[REMAINING_LEN];
        yield_constr.constraint_transition(
            current_remaining_length * (current_remaining_length - next_remaining_length - one),
        );

        // At the start of a sequence, the remaining length must be equal to the starting length minus one
        let sequence_length = vars.local_values[SEQUENCE_LEN];
        yield_constr
            .constraint(sequence_start * (sequence_length - current_remaining_length - one));

        // The remaining length on the last row must be zero.
        yield_constr.constraint_last_row(current_remaining_length);

        // If the current remaining length is zero, the end flag must be one.
        yield_constr.constraint(current_remaining_length * sequence_end);

        // The context, segment and timestamp fields must remain unchanged throughout a byte sequence.
        // The virtual address must decrement by one at each step of a sequence.
        let next_filter = vars.next_values[FILTER];
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
                (sequence_end - one) * (next_byte_index - one) * (next_byte - current_byte),
            );
        }
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        // The filter must be boolean.
        let filter = vars.local_values[FILTER];
        let constraint = builder.mul_sub_extension(filter, filter, filter);
        yield_constr.constraint(builder, constraint);

        // The filter column must start by one.
        let constraint = builder.add_const_extension(filter, F::NEG_ONE);
        yield_constr.constraint_first_row(builder, constraint);

        // The is_read flag must be boolean.
        let is_read = vars.local_values[IS_READ];
        let constraint = builder.mul_sub_extension(is_read, is_read, is_read);
        yield_constr.constraint(builder, constraint);

        // If filter is off, is_read must be off.
        let constraint = builder.mul_sub_extension(is_read, filter, is_read);
        yield_constr.constraint(builder, constraint);

        // Only padding rows have their filter turned off.
        let next_filter = vars.next_values[FILTER];
        let constraint = builder.sub_extension(next_filter, filter);
        let constraint = builder.mul_extension(next_filter, constraint);
        yield_constr.constraint_transition(builder, constraint);

        // The sequence start flag column must start by one.
        let sequence_start = vars.local_values[index_bytes(0)];
        let constraint = builder.add_const_extension(sequence_start, F::NEG_ONE);
        yield_constr.constraint_first_row(builder, constraint);

        // The sequence end flag must be boolean
        let sequence_end = vars.local_values[SEQUENCE_END];
        let constraint = builder.mul_sub_extension(sequence_end, sequence_end, sequence_end);
        yield_constr.constraint(builder, constraint);

        // If the sequence end flag is activated, the next row must be a new sequence or filter must be off.
        let next_sequence_start = vars.next_values[index_bytes(0)];
        let constraint = builder.mul_sub_extension(sequence_end, next_sequence_start, sequence_end);
        let constraint = builder.mul_extension(next_filter, constraint);
        yield_constr.constraint_transition(builder, constraint);

        // Each byte index must be boolean.
        for i in 0..NUM_BYTES {
            let idx_i = vars.local_values[index_bytes(i)];
            let constraint = builder.mul_sub_extension(idx_i, idx_i, idx_i);
            yield_constr.constraint(builder, constraint);
        }

        // There must be only one byte index set to 1 per active row.
        let sum_indices = builder.add_many_extension(
            vars.local_values[index_bytes(0)..index_bytes(0) + NUM_BYTES].into_iter(),
        );
        let constraint = builder.mul_sub_extension(filter, sum_indices, filter);
        yield_constr.constraint(builder, constraint);

        // The remaining length of a byte sequence must decrease by one or be zero.
        let current_remaining_length = vars.local_values[REMAINING_LEN];
        let next_remaining_length = vars.next_values[REMAINING_LEN];
        let length_diff = builder.sub_extension(current_remaining_length, next_remaining_length);
        let length_diff_minus_one = builder.add_const_extension(length_diff, F::NEG_ONE);
        let constraint = builder.mul_extension(current_remaining_length, length_diff_minus_one);
        yield_constr.constraint_transition(builder, constraint);

        // At the start of a sequence, the remaining length must be equal to the starting length minus one
        let sequence_length = vars.local_values[SEQUENCE_LEN];
        let length_diff = builder.sub_extension(sequence_length, current_remaining_length);
        let constraint = builder.mul_sub_extension(sequence_start, length_diff, sequence_start);
        yield_constr.constraint(builder, constraint);

        // The remaining length on the last row must be zero.
        yield_constr.constraint_last_row(builder, current_remaining_length);

        // If the current remaining length is zero, the end flag must be one.
        let constraint = builder.mul_extension(current_remaining_length, sequence_end);
        yield_constr.constraint(builder, constraint);

        // The context, segment and timestamp fields must remain unchanged throughout a byte sequence.
        // The virtual address must decrement by one at each step of a sequence.
        let next_filter = vars.next_values[FILTER];
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
            let constraint = builder.mul_sub_extension(constraint, sequence_end, constraint);
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
