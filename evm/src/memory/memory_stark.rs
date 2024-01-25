use std::marker::PhantomData;

use ethereum_types::U256;
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
use plonky2_maybe_rayon::*;

use super::segments::Segment;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cross_table_lookup::{Column, Filter};
use crate::evaluation_frame::{StarkEvaluationFrame, StarkFrame};
use crate::lookup::Lookup;
use crate::memory::columns::{
    value_limb, ADDR_CONTEXT, ADDR_SEGMENT, ADDR_VIRTUAL, CONTEXT_FIRST_CHANGE, COUNTER, FILTER,
    FREQUENCIES, INITIALIZE_AUX, IS_READ, NUM_COLUMNS, RANGE_CHECK, SEGMENT_FIRST_CHANGE,
    TIMESTAMP, VIRTUAL_FIRST_CHANGE,
};
use crate::memory::VALUE_LIMBS;
use crate::stark::Stark;
use crate::witness::memory::MemoryOpKind::Read;
use crate::witness::memory::{MemoryAddress, MemoryOp};

/// Creates the vector of `Columns` corresponding to:
/// - the memory operation type,
/// - the address in memory of the element being read/written,
/// - the value being read/written,
/// - the timestamp at which the element is read/written.
pub(crate) fn ctl_data<F: Field>() -> Vec<Column<F>> {
    let mut res =
        Column::singles([IS_READ, ADDR_CONTEXT, ADDR_SEGMENT, ADDR_VIRTUAL]).collect_vec();
    res.extend(Column::singles((0..8).map(value_limb)));
    res.push(Column::single(TIMESTAMP));
    res
}

/// CTL filter for memory operations.
pub(crate) fn ctl_filter<F: Field>() -> Filter<F> {
    Filter::new_simple(Column::single(FILTER))
}

#[derive(Copy, Clone, Default)]
pub(crate) struct MemoryStark<F, const D: usize> {
    pub(crate) f: PhantomData<F>,
}

impl MemoryOp {
    /// Generate a row for a given memory operation. Note that this does not generate columns which
    /// depend on the next operation, such as `CONTEXT_FIRST_CHANGE`; those are generated later.
    /// It also does not generate columns such as `COUNTER`, which are generated later, after the
    /// trace has been transposed into column-major form.
    fn into_row<F: Field>(self) -> [F; NUM_COLUMNS] {
        let mut row = [F::ZERO; NUM_COLUMNS];
        row[FILTER] = F::from_bool(self.filter);
        row[TIMESTAMP] = F::from_canonical_usize(self.timestamp);
        row[IS_READ] = F::from_bool(self.kind == Read);
        let MemoryAddress {
            context,
            segment,
            virt,
        } = self.address;
        row[ADDR_CONTEXT] = F::from_canonical_usize(context);
        row[ADDR_SEGMENT] = F::from_canonical_usize(segment);
        row[ADDR_VIRTUAL] = F::from_canonical_usize(virt);
        for j in 0..VALUE_LIMBS {
            row[value_limb(j)] = F::from_canonical_u32((self.value >> (j * 32)).low_u32());
        }
        row
    }
}

/// Generates the `_FIRST_CHANGE` columns and the `RANGE_CHECK` column in the trace.
pub(crate) fn generate_first_change_flags_and_rc<F: RichField>(
    trace_rows: &mut [[F; NUM_COLUMNS]],
) {
    let num_ops = trace_rows.len();
    for idx in 0..num_ops - 1 {
        let row = trace_rows[idx].as_slice();
        let next_row = trace_rows[idx + 1].as_slice();

        let context = row[ADDR_CONTEXT];
        let segment = row[ADDR_SEGMENT];
        let virt = row[ADDR_VIRTUAL];
        let timestamp = row[TIMESTAMP];
        let next_context = next_row[ADDR_CONTEXT];
        let next_segment = next_row[ADDR_SEGMENT];
        let next_virt = next_row[ADDR_VIRTUAL];
        let next_timestamp = next_row[TIMESTAMP];
        let next_is_read = next_row[IS_READ];

        let context_changed = context != next_context;
        let segment_changed = segment != next_segment;
        let virtual_changed = virt != next_virt;

        let context_first_change = context_changed;
        let segment_first_change = segment_changed && !context_first_change;
        let virtual_first_change =
            virtual_changed && !segment_first_change && !context_first_change;

        let row = trace_rows[idx].as_mut_slice();
        row[CONTEXT_FIRST_CHANGE] = F::from_bool(context_first_change);
        row[SEGMENT_FIRST_CHANGE] = F::from_bool(segment_first_change);
        row[VIRTUAL_FIRST_CHANGE] = F::from_bool(virtual_first_change);

        row[RANGE_CHECK] = if context_first_change {
            next_context - context - F::ONE
        } else if segment_first_change {
            next_segment - segment - F::ONE
        } else if virtual_first_change {
            next_virt - virt - F::ONE
        } else {
            next_timestamp - timestamp
        };

        assert!(
            row[RANGE_CHECK].to_canonical_u64() < num_ops as u64,
            "Range check of {} is too large. Bug in fill_gaps?",
            row[RANGE_CHECK]
        );

        let address_changed =
            row[CONTEXT_FIRST_CHANGE] + row[SEGMENT_FIRST_CHANGE] + row[VIRTUAL_FIRST_CHANGE];
        row[INITIALIZE_AUX] = next_segment * address_changed * next_is_read;
    }
}

impl<F: RichField + Extendable<D>, const D: usize> MemoryStark<F, D> {
    /// Generate most of the trace rows. Excludes a few columns like `COUNTER`, which are generated
    /// later, after transposing to column-major form.
    fn generate_trace_row_major(&self, mut memory_ops: Vec<MemoryOp>) -> Vec<[F; NUM_COLUMNS]> {
        // fill_gaps expects an ordered list of operations.
        memory_ops.sort_by_key(MemoryOp::sorting_key);
        Self::fill_gaps(&mut memory_ops);

        Self::pad_memory_ops(&mut memory_ops);

        // fill_gaps may have added operations at the end which break the order, so sort again.
        memory_ops.sort_by_key(MemoryOp::sorting_key);

        let mut trace_rows = memory_ops
            .into_par_iter()
            .map(|op| op.into_row())
            .collect::<Vec<_>>();
        generate_first_change_flags_and_rc(trace_rows.as_mut_slice());
        trace_rows
    }

    /// Generates the `COUNTER`, `RANGE_CHECK` and `FREQUENCIES` columns, given a
    /// trace in column-major form.
    fn generate_trace_col_major(trace_col_vecs: &mut [Vec<F>]) {
        let height = trace_col_vecs[0].len();
        trace_col_vecs[COUNTER] = (0..height).map(|i| F::from_canonical_usize(i)).collect();

        for i in 0..height {
            let x_rc = trace_col_vecs[RANGE_CHECK][i].to_canonical_u64() as usize;
            trace_col_vecs[FREQUENCIES][x_rc] += F::ONE;
            if (trace_col_vecs[CONTEXT_FIRST_CHANGE][i] == F::ONE)
                || (trace_col_vecs[SEGMENT_FIRST_CHANGE][i] == F::ONE)
            {
                // CONTEXT_FIRST_CHANGE and SEGMENT_FIRST_CHANGE should be 0 at the last row, so the index
                // should never be out of bounds.
                let x_fo = trace_col_vecs[ADDR_VIRTUAL][i + 1].to_canonical_u64() as usize;
                trace_col_vecs[FREQUENCIES][x_fo] += F::ONE;
            }
        }
    }

    /// This memory STARK orders rows by `(context, segment, virt, timestamp)`. To enforce the
    /// ordering, it range checks the delta of the first field that changed.
    ///
    /// This method adds some dummy operations to ensure that none of these range checks will be too
    /// large, i.e. that they will all be smaller than the number of rows, allowing them to be
    /// checked easily with a single lookup.
    ///
    /// For example, say there are 32 memory operations, and a particular address is accessed at
    /// timestamps 20 and 100. 80 would fail the range check, so this method would add two dummy
    /// reads to the same address, say at timestamps 50 and 80.
    fn fill_gaps(memory_ops: &mut Vec<MemoryOp>) {
        let max_rc = memory_ops.len().next_power_of_two() - 1;
        for (mut curr, mut next) in memory_ops.clone().into_iter().tuple_windows() {
            if curr.address.context != next.address.context
                || curr.address.segment != next.address.segment
            {
                // We won't bother to check if there's a large context gap, because there can't be
                // more than 500 contexts or so, as explained here:
                // https://notes.ethereum.org/@vbuterin/proposals_to_adjust_memory_gas_costs
                // Similarly, the number of possible segments is a small constant, so any gap must
                // be small. max_rc will always be much larger, as just bootloading the kernel will
                // trigger thousands of memory operations.
                // However, we do check that the first address accessed is range-checkable. If not,
                // we could start at a negative address and cheat.
                while next.address.virt > max_rc {
                    let mut dummy_address = next.address;
                    dummy_address.virt -= max_rc;
                    let dummy_read = MemoryOp::new_dummy_read(dummy_address, 0, U256::zero());
                    memory_ops.push(dummy_read);
                    next = dummy_read;
                }
            } else if curr.address.virt != next.address.virt {
                while next.address.virt - curr.address.virt - 1 > max_rc {
                    let mut dummy_address = curr.address;
                    dummy_address.virt += max_rc + 1;
                    let dummy_read = MemoryOp::new_dummy_read(dummy_address, 0, U256::zero());
                    memory_ops.push(dummy_read);
                    curr = dummy_read;
                }
            } else {
                while next.timestamp - curr.timestamp > max_rc {
                    let dummy_read =
                        MemoryOp::new_dummy_read(curr.address, curr.timestamp + max_rc, curr.value);
                    memory_ops.push(dummy_read);
                    curr = dummy_read;
                }
            }
        }
    }

    fn pad_memory_ops(memory_ops: &mut Vec<MemoryOp>) {
        let last_op = *memory_ops.last().expect("No memory ops?");

        // We essentially repeat the last operation until our operation list has the desired size,
        // with a few changes:
        // - We change its filter to 0 to indicate that this is a dummy operation.
        // - We make sure it's a read, since dummy operations must be reads.
        let padding_op = MemoryOp {
            filter: false,
            kind: Read,
            ..last_op
        };

        let num_ops = memory_ops.len();
        let num_ops_padded = num_ops.next_power_of_two();
        for _ in num_ops..num_ops_padded {
            memory_ops.push(padding_op);
        }
    }

    pub(crate) fn generate_trace(
        &self,
        memory_ops: Vec<MemoryOp>,
        timing: &mut TimingTree,
    ) -> Vec<PolynomialValues<F>> {
        // Generate most of the trace in row-major form.
        let trace_rows = timed!(
            timing,
            "generate trace rows",
            self.generate_trace_row_major(memory_ops)
        );
        let trace_row_vecs: Vec<_> = trace_rows.into_iter().map(|row| row.to_vec()).collect();

        // Transpose to column-major form.
        let mut trace_col_vecs = transpose(&trace_row_vecs);

        // A few final generation steps, which work better in column-major form.
        Self::generate_trace_col_major(&mut trace_col_vecs);

        trace_col_vecs
            .into_iter()
            .map(|column| PolynomialValues::new(column))
            .collect()
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Stark<F, D> for MemoryStark<F, D> {
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
        let one = P::from(FE::ONE);
        let local_values = vars.get_local_values();
        let next_values = vars.get_next_values();

        let timestamp = local_values[TIMESTAMP];
        let addr_context = local_values[ADDR_CONTEXT];
        let addr_segment = local_values[ADDR_SEGMENT];
        let addr_virtual = local_values[ADDR_VIRTUAL];
        let value_limbs: Vec<_> = (0..8).map(|i| local_values[value_limb(i)]).collect();

        let next_timestamp = next_values[TIMESTAMP];
        let next_is_read = next_values[IS_READ];
        let next_addr_context = next_values[ADDR_CONTEXT];
        let next_addr_segment = next_values[ADDR_SEGMENT];
        let next_addr_virtual = next_values[ADDR_VIRTUAL];
        let next_values_limbs: Vec<_> = (0..8).map(|i| next_values[value_limb(i)]).collect();

        // The filter must be 0 or 1.
        let filter = local_values[FILTER];
        yield_constr.constraint(filter * (filter - P::ONES));

        // IS_READ must be 0 or 1.
        // This is implied by the MemoryStark CTL, where corresponding values are either
        // hardcoded to 0/1, or boolean-constrained in their respective STARK modules.

        // If this is a dummy row (filter is off), it must be a read. This means the prover can
        // insert reads which never appear in the CPU trace (which are harmless), but not writes.
        let is_dummy = P::ONES - filter;
        let is_write = P::ONES - local_values[IS_READ];
        yield_constr.constraint(is_dummy * is_write);

        let context_first_change = local_values[CONTEXT_FIRST_CHANGE];
        let segment_first_change = local_values[SEGMENT_FIRST_CHANGE];
        let virtual_first_change = local_values[VIRTUAL_FIRST_CHANGE];
        let address_unchanged =
            one - context_first_change - segment_first_change - virtual_first_change;

        let range_check = local_values[RANGE_CHECK];

        let not_context_first_change = one - context_first_change;
        let not_segment_first_change = one - segment_first_change;
        let not_virtual_first_change = one - virtual_first_change;
        let not_address_unchanged = one - address_unchanged;

        // First set of ordering constraint: first_change flags are boolean.
        yield_constr.constraint(context_first_change * not_context_first_change);
        yield_constr.constraint(segment_first_change * not_segment_first_change);
        yield_constr.constraint(virtual_first_change * not_virtual_first_change);
        yield_constr.constraint(address_unchanged * not_address_unchanged);

        // Second set of ordering constraints: no change before the column corresponding to the nonzero first_change flag.
        yield_constr
            .constraint_transition(segment_first_change * (next_addr_context - addr_context));
        yield_constr
            .constraint_transition(virtual_first_change * (next_addr_context - addr_context));
        yield_constr
            .constraint_transition(virtual_first_change * (next_addr_segment - addr_segment));
        yield_constr.constraint_transition(address_unchanged * (next_addr_context - addr_context));
        yield_constr.constraint_transition(address_unchanged * (next_addr_segment - addr_segment));
        yield_constr.constraint_transition(address_unchanged * (next_addr_virtual - addr_virtual));

        // Third set of ordering constraints: range-check difference in the column that should be increasing.
        let computed_range_check = context_first_change * (next_addr_context - addr_context - one)
            + segment_first_change * (next_addr_segment - addr_segment - one)
            + virtual_first_change * (next_addr_virtual - addr_virtual - one)
            + address_unchanged * (next_timestamp - timestamp);
        yield_constr.constraint_transition(range_check - computed_range_check);

        // Validate initialize_aux. It contains next_segment * addr_changed * next_is_read.
        let initialize_aux = local_values[INITIALIZE_AUX];
        yield_constr.constraint_transition(
            initialize_aux - next_addr_segment * not_address_unchanged * next_is_read,
        );

        for i in 0..8 {
            // Enumerate purportedly-ordered log.
            yield_constr.constraint_transition(
                next_is_read * address_unchanged * (next_values_limbs[i] - value_limbs[i]),
            );
            // By default, memory is initialized with 0. This means that if the first operation of a new address is a read,
            // then its value must be 0.
            // There are exceptions, though: this constraint zero-initializes everything but the code segment and context 0.
            yield_constr
                .constraint_transition(next_addr_context * initialize_aux * next_values_limbs[i]);
            // We don't want to exclude the entirety of context 0. This constraint zero-initializes all segments except the
            // specified ones (segment 0 is already included in initialize_aux).
            // There is overlap with the previous constraint, but this is not a problem.
            yield_constr.constraint_transition(
                (next_addr_segment - P::Scalar::from_canonical_usize(Segment::TrieData.unscale()))
                    * initialize_aux
                    * next_values_limbs[i],
            );
        }

        // Check the range column: First value must be 0,
        // and intermediate rows must increment by 1.
        let rc1 = local_values[COUNTER];
        let rc2 = next_values[COUNTER];
        yield_constr.constraint_first_row(rc1);
        let incr = rc2 - rc1;
        yield_constr.constraint_transition(incr - P::Scalar::ONES);
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: &Self::EvaluationFrameTarget,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let one = builder.one_extension();
        let local_values = vars.get_local_values();
        let next_values = vars.get_next_values();

        let addr_context = local_values[ADDR_CONTEXT];
        let addr_segment = local_values[ADDR_SEGMENT];
        let addr_virtual = local_values[ADDR_VIRTUAL];
        let value_limbs: Vec<_> = (0..8).map(|i| local_values[value_limb(i)]).collect();
        let timestamp = local_values[TIMESTAMP];

        let next_addr_context = next_values[ADDR_CONTEXT];
        let next_addr_segment = next_values[ADDR_SEGMENT];
        let next_addr_virtual = next_values[ADDR_VIRTUAL];
        let next_values_limbs: Vec<_> = (0..8).map(|i| next_values[value_limb(i)]).collect();
        let next_is_read = next_values[IS_READ];
        let next_timestamp = next_values[TIMESTAMP];

        // The filter must be 0 or 1.
        let filter = local_values[FILTER];
        let constraint = builder.mul_sub_extension(filter, filter, filter);
        yield_constr.constraint(builder, constraint);

        // IS_READ must be 0 or 1.
        // This is implied by the MemoryStark CTL, where corresponding values are either
        // hardcoded to 0/1, or boolean-constrained in their respective STARK modules.

        // If this is a dummy row (filter is off), it must be a read. This means the prover can
        // insert reads which never appear in the CPU trace (which are harmless), but not writes.
        let is_dummy = builder.sub_extension(one, filter);
        let is_write = builder.sub_extension(one, local_values[IS_READ]);
        let is_dummy_write = builder.mul_extension(is_dummy, is_write);
        yield_constr.constraint(builder, is_dummy_write);

        let context_first_change = local_values[CONTEXT_FIRST_CHANGE];
        let segment_first_change = local_values[SEGMENT_FIRST_CHANGE];
        let virtual_first_change = local_values[VIRTUAL_FIRST_CHANGE];
        let address_unchanged = {
            let mut cur = builder.sub_extension(one, context_first_change);
            cur = builder.sub_extension(cur, segment_first_change);
            builder.sub_extension(cur, virtual_first_change)
        };

        let range_check = local_values[RANGE_CHECK];

        let not_context_first_change = builder.sub_extension(one, context_first_change);
        let not_segment_first_change = builder.sub_extension(one, segment_first_change);
        let not_virtual_first_change = builder.sub_extension(one, virtual_first_change);
        let not_address_unchanged = builder.sub_extension(one, address_unchanged);
        let addr_context_diff = builder.sub_extension(next_addr_context, addr_context);
        let addr_segment_diff = builder.sub_extension(next_addr_segment, addr_segment);
        let addr_virtual_diff = builder.sub_extension(next_addr_virtual, addr_virtual);

        // First set of ordering constraint: traces are boolean.
        let context_first_change_bool =
            builder.mul_extension(context_first_change, not_context_first_change);
        yield_constr.constraint(builder, context_first_change_bool);
        let segment_first_change_bool =
            builder.mul_extension(segment_first_change, not_segment_first_change);
        yield_constr.constraint(builder, segment_first_change_bool);
        let virtual_first_change_bool =
            builder.mul_extension(virtual_first_change, not_virtual_first_change);
        yield_constr.constraint(builder, virtual_first_change_bool);
        let address_unchanged_bool =
            builder.mul_extension(address_unchanged, not_address_unchanged);
        yield_constr.constraint(builder, address_unchanged_bool);

        // Second set of ordering constraints: no change before the column corresponding to the nonzero first_change flag.
        let segment_first_change_check =
            builder.mul_extension(segment_first_change, addr_context_diff);
        yield_constr.constraint_transition(builder, segment_first_change_check);
        let virtual_first_change_check_1 =
            builder.mul_extension(virtual_first_change, addr_context_diff);
        yield_constr.constraint_transition(builder, virtual_first_change_check_1);
        let virtual_first_change_check_2 =
            builder.mul_extension(virtual_first_change, addr_segment_diff);
        yield_constr.constraint_transition(builder, virtual_first_change_check_2);
        let address_unchanged_check_1 = builder.mul_extension(address_unchanged, addr_context_diff);
        yield_constr.constraint_transition(builder, address_unchanged_check_1);
        let address_unchanged_check_2 = builder.mul_extension(address_unchanged, addr_segment_diff);
        yield_constr.constraint_transition(builder, address_unchanged_check_2);
        let address_unchanged_check_3 = builder.mul_extension(address_unchanged, addr_virtual_diff);
        yield_constr.constraint_transition(builder, address_unchanged_check_3);

        // Third set of ordering constraints: range-check difference in the column that should be increasing.
        let context_diff = {
            let diff = builder.sub_extension(next_addr_context, addr_context);
            builder.sub_extension(diff, one)
        };
        let segment_diff = {
            let diff = builder.sub_extension(next_addr_segment, addr_segment);
            builder.sub_extension(diff, one)
        };
        let segment_range_check = builder.mul_extension(segment_first_change, segment_diff);
        let virtual_diff = {
            let diff = builder.sub_extension(next_addr_virtual, addr_virtual);
            builder.sub_extension(diff, one)
        };
        let virtual_range_check = builder.mul_extension(virtual_first_change, virtual_diff);
        let timestamp_diff = builder.sub_extension(next_timestamp, timestamp);
        let timestamp_range_check = builder.mul_extension(address_unchanged, timestamp_diff);

        let computed_range_check = {
            // context_range_check = context_first_change * context_diff
            let mut sum =
                builder.mul_add_extension(context_first_change, context_diff, segment_range_check);
            sum = builder.add_extension(sum, virtual_range_check);
            builder.add_extension(sum, timestamp_range_check)
        };
        let range_check_diff = builder.sub_extension(range_check, computed_range_check);
        yield_constr.constraint_transition(builder, range_check_diff);

        // Validate initialize_aux. It contains next_segment * addr_changed * next_is_read.
        let initialize_aux = local_values[INITIALIZE_AUX];
        let computed_initialize_aux = builder.mul_extension(not_address_unchanged, next_is_read);
        let computed_initialize_aux =
            builder.mul_extension(next_addr_segment, computed_initialize_aux);
        let new_first_read_constraint =
            builder.sub_extension(initialize_aux, computed_initialize_aux);
        yield_constr.constraint_transition(builder, new_first_read_constraint);

        for i in 0..8 {
            // Enumerate purportedly-ordered log.
            let value_diff = builder.sub_extension(next_values_limbs[i], value_limbs[i]);
            let zero_if_read = builder.mul_extension(address_unchanged, value_diff);
            let read_constraint = builder.mul_extension(next_is_read, zero_if_read);
            yield_constr.constraint_transition(builder, read_constraint);
            // By default, memory is initialized with 0. This means that if the first operation of a new address is a read,
            // then its value must be 0.
            // There are exceptions, though: this constraint zero-initializes everything but the code segment and context 0.
            let context_zero_initializing_constraint =
                builder.mul_extension(next_values_limbs[i], initialize_aux);
            let initializing_constraint =
                builder.mul_extension(next_addr_context, context_zero_initializing_constraint);
            yield_constr.constraint_transition(builder, initializing_constraint);
            // We don't want to exclude the entirety of context 0. This constraint zero-initializes all segments except the
            // specified ones (segment 0 is already included in initialize_aux).
            // There is overlap with the previous constraint, but this is not a problem.
            let segment_trie_data = builder.add_const_extension(
                next_addr_segment,
                F::NEG_ONE * F::from_canonical_usize(Segment::TrieData.unscale()),
            );
            let zero_init_constraint =
                builder.mul_extension(segment_trie_data, context_zero_initializing_constraint);
            yield_constr.constraint_transition(builder, zero_init_constraint);
        }

        // Check the range column: First value must be 0,
        // and intermediate rows must increment by 1.
        let rc1 = local_values[COUNTER];
        let rc2 = next_values[COUNTER];
        yield_constr.constraint_first_row(builder, rc1);
        let incr = builder.sub_extension(rc2, rc1);
        let t = builder.sub_extension(incr, one);
        yield_constr.constraint_transition(builder, t);
    }

    fn constraint_degree(&self) -> usize {
        3
    }

    fn lookups(&self) -> Vec<Lookup<F>> {
        vec![Lookup {
            columns: vec![
                Column::single(RANGE_CHECK),
                Column::single_next_row(ADDR_VIRTUAL),
            ],
            table_column: Column::single(COUNTER),
            frequencies_column: Column::single(FREQUENCIES),
            filter_columns: vec![
                None,
                Some(Filter::new_simple(Column::sum([
                    CONTEXT_FIRST_CHANGE,
                    SEGMENT_FIRST_CHANGE,
                ]))),
            ],
        }]
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use anyhow::Result;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    use crate::memory::memory_stark::MemoryStark;
    use crate::stark_testing::{test_stark_circuit_constraints, test_stark_low_degree};

    #[test]
    fn test_stark_degree() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        type S = MemoryStark<F, D>;

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
        type S = MemoryStark<F, D>;

        let stark = S {
            f: Default::default(),
        };
        test_stark_circuit_constraints::<F, C, S, D>(stark)
    }
}
