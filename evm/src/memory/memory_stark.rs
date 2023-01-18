use std::marker::PhantomData;

use itertools::Itertools;
use maybe_rayon::*;
use plonky2::field::extension::{Extendable, FieldExtension};
use plonky2::field::packed::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::timed;
use plonky2::util::timing::TimingTree;
use plonky2::util::transpose;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cross_table_lookup::Column;
use crate::lookup::{eval_lookups, eval_lookups_circuit, permuted_cols};
use crate::memory::columns::{
    value_limb, ADDR_CONTEXT, ADDR_SEGMENT, ADDR_VIRTUAL, CONTEXT_FIRST_CHANGE, COUNTER,
    COUNTER_PERMUTED, FILTER, IS_READ, NUM_COLUMNS, RANGE_CHECK, RANGE_CHECK_PERMUTED,
    SEGMENT_FIRST_CHANGE, TIMESTAMP, VIRTUAL_FIRST_CHANGE,
};
use crate::memory::VALUE_LIMBS;
use crate::permutation::PermutationPair;
use crate::stark::Stark;
use crate::vars::{StarkEvaluationTargets, StarkEvaluationVars};
use crate::witness::memory::MemoryOpKind::Read;
use crate::witness::memory::{MemoryAddress, MemoryOp};

pub fn ctl_data<F: Field>() -> Vec<Column<F>> {
    let mut res =
        Column::singles([IS_READ, ADDR_CONTEXT, ADDR_SEGMENT, ADDR_VIRTUAL]).collect_vec();
    res.extend(Column::singles((0..8).map(value_limb)));
    res.push(Column::single(TIMESTAMP));
    res
}

pub fn ctl_filter<F: Field>() -> Column<F> {
    Column::single(FILTER)
}

#[derive(Copy, Clone, Default)]
pub struct MemoryStark<F, const D: usize> {
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
pub fn generate_first_change_flags_and_rc<F: RichField>(trace_rows: &mut [[F; NUM_COLUMNS]]) {
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

    /// Generates the `COUNTER`, `RANGE_CHECK_PERMUTED` and `COUNTER_PERMUTED` columns, given a
    /// trace in column-major form.
    fn generate_trace_col_major(trace_col_vecs: &mut [Vec<F>]) {
        let height = trace_col_vecs[0].len();
        trace_col_vecs[COUNTER] = (0..height).map(|i| F::from_canonical_usize(i)).collect();

        let (permuted_inputs, permuted_table) =
            permuted_cols(&trace_col_vecs[RANGE_CHECK], &trace_col_vecs[COUNTER]);
        trace_col_vecs[RANGE_CHECK_PERMUTED] = permuted_inputs;
        trace_col_vecs[COUNTER_PERMUTED] = permuted_table;
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
        for (mut curr, next) in memory_ops.clone().into_iter().tuple_windows() {
            if curr.address.context != next.address.context
                || curr.address.segment != next.address.segment
            {
                // We won't bother to check if there's a large context gap, because there can't be
                // more than 500 contexts or so, as explained here:
                // https://notes.ethereum.org/@vbuterin/proposals_to_adjust_memory_gas_costs
                // Similarly, the number of possible segments is a small constant, so any gap must
                // be small. max_rc will always be much larger, as just bootloading the kernel will
                // trigger thousands of memory operations.
            } else if curr.address.virt != next.address.virt {
                while next.address.virt - curr.address.virt - 1 > max_rc {
                    let mut dummy_address = curr.address;
                    dummy_address.virt += max_rc + 1;
                    let dummy_read = MemoryOp::new_dummy_read(dummy_address, 0);
                    memory_ops.push(dummy_read);
                    curr = dummy_read;
                }
            } else {
                while next.timestamp - curr.timestamp > max_rc {
                    let dummy_read =
                        MemoryOp::new_dummy_read(curr.address, curr.timestamp + max_rc);
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
    const COLUMNS: usize = NUM_COLUMNS;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, { Self::COLUMNS }>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        let one = P::from(FE::ONE);

        let timestamp = vars.local_values[TIMESTAMP];
        let addr_context = vars.local_values[ADDR_CONTEXT];
        let addr_segment = vars.local_values[ADDR_SEGMENT];
        let addr_virtual = vars.local_values[ADDR_VIRTUAL];
        let values: Vec<_> = (0..8).map(|i| vars.local_values[value_limb(i)]).collect();

        let next_timestamp = vars.next_values[TIMESTAMP];
        let next_is_read = vars.next_values[IS_READ];
        let next_addr_context = vars.next_values[ADDR_CONTEXT];
        let next_addr_segment = vars.next_values[ADDR_SEGMENT];
        let next_addr_virtual = vars.next_values[ADDR_VIRTUAL];
        let next_values: Vec<_> = (0..8).map(|i| vars.next_values[value_limb(i)]).collect();

        // The filter must be 0 or 1.
        let filter = vars.local_values[FILTER];
        yield_constr.constraint(filter * (filter - P::ONES));

        // If this is a dummy row (filter is off), it must be a read. This means the prover can
        // insert reads which never appear in the CPU trace (which are harmless), but not writes.
        let is_dummy = P::ONES - filter;
        let is_write = P::ONES - vars.local_values[IS_READ];
        yield_constr.constraint(is_dummy * is_write);

        let context_first_change = vars.local_values[CONTEXT_FIRST_CHANGE];
        let segment_first_change = vars.local_values[SEGMENT_FIRST_CHANGE];
        let virtual_first_change = vars.local_values[VIRTUAL_FIRST_CHANGE];
        let address_unchanged =
            one - context_first_change - segment_first_change - virtual_first_change;

        let range_check = vars.local_values[RANGE_CHECK];

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

        // Enumerate purportedly-ordered log.
        for i in 0..8 {
            yield_constr
                .constraint(next_is_read * address_unchanged * (next_values[i] - values[i]));
        }

        eval_lookups(vars, yield_constr, RANGE_CHECK_PERMUTED, COUNTER_PERMUTED)
    }

    fn eval_ext_circuit(
        &self,
        builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, { Self::COLUMNS }>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let one = builder.one_extension();

        let addr_context = vars.local_values[ADDR_CONTEXT];
        let addr_segment = vars.local_values[ADDR_SEGMENT];
        let addr_virtual = vars.local_values[ADDR_VIRTUAL];
        let values: Vec<_> = (0..8).map(|i| vars.local_values[value_limb(i)]).collect();
        let timestamp = vars.local_values[TIMESTAMP];

        let next_addr_context = vars.next_values[ADDR_CONTEXT];
        let next_addr_segment = vars.next_values[ADDR_SEGMENT];
        let next_addr_virtual = vars.next_values[ADDR_VIRTUAL];
        let next_values: Vec<_> = (0..8).map(|i| vars.next_values[value_limb(i)]).collect();
        let next_is_read = vars.next_values[IS_READ];
        let next_timestamp = vars.next_values[TIMESTAMP];

        // The filter must be 0 or 1.
        let filter = vars.local_values[FILTER];
        let constraint = builder.mul_sub_extension(filter, filter, filter);
        yield_constr.constraint(builder, constraint);

        // If this is a dummy row (filter is off), it must be a read. This means the prover can
        // insert reads which never appear in the CPU trace (which are harmless), but not writes.
        let is_dummy = builder.sub_extension(one, filter);
        let is_write = builder.sub_extension(one, vars.local_values[IS_READ]);
        let is_dummy_write = builder.mul_extension(is_dummy, is_write);
        yield_constr.constraint(builder, is_dummy_write);

        let context_first_change = vars.local_values[CONTEXT_FIRST_CHANGE];
        let segment_first_change = vars.local_values[SEGMENT_FIRST_CHANGE];
        let virtual_first_change = vars.local_values[VIRTUAL_FIRST_CHANGE];
        let address_unchanged = {
            let mut cur = builder.sub_extension(one, context_first_change);
            cur = builder.sub_extension(cur, segment_first_change);
            builder.sub_extension(cur, virtual_first_change)
        };

        let range_check = vars.local_values[RANGE_CHECK];

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
        let context_range_check = builder.mul_extension(context_first_change, context_diff);
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
            let mut sum = builder.add_extension(context_range_check, segment_range_check);
            sum = builder.add_extension(sum, virtual_range_check);
            builder.add_extension(sum, timestamp_range_check)
        };
        let range_check_diff = builder.sub_extension(range_check, computed_range_check);
        yield_constr.constraint_transition(builder, range_check_diff);

        // Enumerate purportedly-ordered log.
        for i in 0..8 {
            let value_diff = builder.sub_extension(next_values[i], values[i]);
            let zero_if_read = builder.mul_extension(address_unchanged, value_diff);
            let read_constraint = builder.mul_extension(next_is_read, zero_if_read);
            yield_constr.constraint(builder, read_constraint);
        }

        eval_lookups_circuit(
            builder,
            vars,
            yield_constr,
            RANGE_CHECK_PERMUTED,
            COUNTER_PERMUTED,
        )
    }

    fn constraint_degree(&self) -> usize {
        3
    }

    fn permutation_pairs(&self) -> Vec<PermutationPair> {
        vec![
            PermutationPair::singletons(RANGE_CHECK, RANGE_CHECK_PERMUTED),
            PermutationPair::singletons(COUNTER, COUNTER_PERMUTED),
        ]
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
