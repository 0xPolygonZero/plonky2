//! Implementation of the Halo2 lookup argument.
//!
//! References:
//! - https://zcash.github.io/halo2/design/proving-system/lookup.html
//! - https://www.youtube.com/watch?v=YlTt12s7vGE&t=5237s

use std::cmp::Ordering;

use itertools::Itertools;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::{Field, PrimeField64};
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use starky::vars::{StarkEvaluationTargets, StarkEvaluationVars};

use crate::public_input_layout::NUM_PUBLIC_INPUTS;
use crate::registers::lookup::*;
use crate::registers::NUM_COLUMNS;

pub(crate) fn generate_lookups<F: PrimeField64>(trace_cols: &mut [Vec<F>]) {
    for i in 0..NUM_LOOKUPS {
        let inputs = &trace_cols[col_input(i)];
        let table = &trace_cols[col_table(i)];
        let (permuted_inputs, permuted_table) = permuted_cols(inputs, table);
        trace_cols[col_permuted_input(i)] = permuted_inputs;
        trace_cols[col_permuted_table(i)] = permuted_table;
    }
}

/// Given an input column and a table column, generate the permuted input and permuted table columns
/// used in the Halo2 permutation argument.
pub fn permuted_cols<F: PrimeField64>(inputs: &[F], table: &[F]) -> (Vec<F>, Vec<F>) {
    let n = inputs.len();

    // The permuted inputs do not have to be ordered, but we found that sorting was faster than
    // hash-based grouping. We also sort the table, as this helps us identify "unused" table
    // elements efficiently.

    // To compare elements, e.g. for sorting, we first need them in canonical form. It would be
    // wasteful to canonicalize in each comparison, as a single element may be involved in many
    // comparisons. So we will canonicalize once upfront, then use `to_noncanonical_u64` when
    // comparing elements.

    let sorted_inputs = inputs
        .iter()
        .map(|x| x.to_canonical())
        .sorted_unstable_by_key(|x| x.to_noncanonical_u64())
        .collect_vec();
    let sorted_table = table
        .iter()
        .map(|x| x.to_canonical())
        .sorted_unstable_by_key(|x| x.to_noncanonical_u64())
        .collect_vec();

    let mut unused_table_inds = Vec::with_capacity(n);
    let mut unused_table_vals = Vec::with_capacity(n);
    let mut permuted_table = vec![F::ZERO; n];
    let mut i = 0;
    let mut j = 0;
    while (j < n) && (i < n) {
        let input_val = sorted_inputs[i].to_noncanonical_u64();
        let table_val = sorted_table[j].to_noncanonical_u64();
        match input_val.cmp(&table_val) {
            Ordering::Greater => {
                unused_table_vals.push(sorted_table[j]);
                j += 1;
            }
            Ordering::Less => {
                if let Some(x) = unused_table_vals.pop() {
                    permuted_table[i] = x;
                } else {
                    unused_table_inds.push(i);
                }
                i += 1;
            }
            Ordering::Equal => {
                permuted_table[i] = sorted_table[j];
                i += 1;
                j += 1;
            }
        }
    }

    #[allow(clippy::needless_range_loop)] // indexing is just more natural here
    for jj in j..n {
        unused_table_vals.push(sorted_table[jj]);
    }
    for ii in i..n {
        unused_table_inds.push(ii);
    }
    for (ind, val) in unused_table_inds.into_iter().zip_eq(unused_table_vals) {
        permuted_table[ind] = val;
    }

    (sorted_inputs, permuted_table)
}

pub(crate) fn eval_lookups<F: Field, P: PackedField<Scalar = F>>(
    vars: StarkEvaluationVars<F, P, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    for i in 0..NUM_LOOKUPS {
        let local_perm_input = vars.local_values[col_permuted_input(i)];
        let next_perm_table = vars.next_values[col_permuted_table(i)];
        let next_perm_input = vars.next_values[col_permuted_input(i)];

        // A "vertical" diff between the local and next permuted inputs.
        let diff_input_prev = next_perm_input - local_perm_input;
        // A "horizontal" diff between the next permuted input and permuted table value.
        let diff_input_table = next_perm_input - next_perm_table;

        yield_constr.constraint(diff_input_prev * diff_input_table);

        // This is actually constraining the first row, as per the spec, since `diff_input_table`
        // is a diff of the next row's values. In the context of `constraint_last_row`, the next
        // row is the first row.
        yield_constr.constraint_last_row(diff_input_table);
    }
}

pub(crate) fn eval_lookups_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    vars: StarkEvaluationTargets<D, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    for i in 0..NUM_LOOKUPS {
        let local_perm_input = vars.local_values[col_permuted_input(i)];
        let next_perm_table = vars.next_values[col_permuted_table(i)];
        let next_perm_input = vars.next_values[col_permuted_input(i)];

        // A "vertical" diff between the local and next permuted inputs.
        let diff_input_prev = builder.sub_extension(next_perm_input, local_perm_input);
        // A "horizontal" diff between the next permuted input and permuted table value.
        let diff_input_table = builder.sub_extension(next_perm_input, next_perm_table);

        let diff_product = builder.mul_extension(diff_input_prev, diff_input_table);
        yield_constr.constraint(builder, diff_product);

        // This is actually constraining the first row, as per the spec, since `diff_input_table`
        // is a diff of the next row's values. In the context of `constraint_last_row`, the next
        // row is the first row.
        yield_constr.constraint_last_row(builder, diff_input_table);
    }
}
