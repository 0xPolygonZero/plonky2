//! Implementation of the Halo2 lookup argument.
//!
//! References:
//! - https://zcash.github.io/halo2/design/proving-system/lookup.html
//! - https://www.youtube.com/watch?v=YlTt12s7vGE&t=5237s

use std::cmp::Ordering;

use itertools::Itertools;
use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::{Field, PrimeField64};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use starky::vars::StarkEvaluationTargets;
use starky::vars::StarkEvaluationVars;

use crate::public_input_layout::NUM_PUBLIC_INPUTS;
use crate::registers::lookup::*;
use crate::registers::NUM_COLUMNS;
use crate::util::{create_hash_bag, flatten_hash_bag};

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

    // In the permuted inputs, copies of the same value must be grouped together. We accomplish this
    // by building a hash-based bag (aka multiset) from the inputs, then flattening it.
    let input_bag = create_hash_bag(inputs);
    let permuted_inputs = flatten_hash_bag(&input_bag);

    // Enumerate tables values that do not appear in the input list.
    let mut unused_table_vals = table.iter().filter(|v| !input_bag.contains_key(v)).copied();

    // Build the permuted table while enumerating permuted inputs. If a permuted input is a repeat,
    // we place an unused table value, otherwise we place the permuted input value.
    let mut permuted_table = Vec::with_capacity(n);
    permuted_table.push(permuted_inputs[0]);
    for i in 1..n {
        let is_repeat = permuted_inputs[i] == permuted_inputs[i - 1];
        permuted_table.push(if is_repeat {
            unused_table_vals
                .next()
                .expect("No more unused table values; this should never happen")
        } else {
            permuted_inputs[i]
        });
    }

    assert_eq!(
        unused_table_vals.next(),
        None,
        "Extra unused table values; this means some value(s) were not in the table"
    );

    (permuted_inputs, permuted_table)
}

pub fn permuted_cols_v2<F: PrimeField64>(inputs: &[F], table: &[F]) -> (Vec<F>, Vec<F>) {
    let n = inputs.len();

    // We will canonicalize the elements of sorted_inputs and sorted_table once upfront, then use
    // `to_noncanonical_u64` when comparing elements later.
    let mut sorted_inputs = inputs
        .iter()
        .map(|x| x.to_canonical())
        .sorted_unstable_by_key(|x| x.to_noncanonical_u64())
        .collect_vec();
    let mut sorted_table = table
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
        match sorted_inputs[i]
            .to_noncanonical_u64()
            .cmp(&sorted_table[j].to_noncanonical_u64())
        {
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

        // A "vertical" diff between this permuted input and the one in the previous row.
        let diff_input_prev = next_perm_input - local_perm_input;
        // A "horizontal" diff between this permuted input and the associated permuted table value.
        let diff_input_table = next_perm_input - next_perm_table;

        yield_constr.constraint(diff_input_prev * diff_input_table);

        // This is actually constraining the first row, as per the spec, since `diff_input_table`
        // is a diff of the next row's values. In the context of `constraint_last_row`, the next
        // row is the first row.
        yield_constr.constraint_last_row(diff_input_table);
    }
}

pub(crate) fn eval_lookups_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    vars: StarkEvaluationTargets<D, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    for i in 0..NUM_LOOKUPS {
        let local_perm_input = vars.local_values[col_permuted_input(i)];
        let next_perm_table = vars.next_values[col_permuted_table(i)];
        let next_perm_input = vars.next_values[col_permuted_input(i)];

        // A "vertical" diff between this permuted input and the one in the previous row.
        let diff_input_prev = builder.sub_extension(next_perm_input, local_perm_input);
        // A "horizontal" diff between this permuted input and the associated permuted table value.
        let diff_input_table = builder.sub_extension(next_perm_input, next_perm_table);

        let diff_product = builder.mul_extension(diff_input_prev, diff_input_table);
        yield_constr.constraint(builder, diff_product);

        // This is actually constraining the first row, as per the spec, since `diff_input_table`
        // is a diff of the next row's values. In the context of `constraint_last_row`, the next
        // row is the first row.
        yield_constr.constraint_last_row(builder, diff_input_table);
    }
}
