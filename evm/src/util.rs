use std::cmp::Ordering;

use itertools::Itertools;
use plonky2::field::field_types::{Field, PrimeField64};
use plonky2::field::polynomial::PolynomialValues;
use plonky2::util::transpose;

/// A helper function to transpose a row-wise trace and put it in the format that `prove` expects.
pub fn trace_rows_to_poly_values<F: Field, const COLUMNS: usize>(
    trace_rows: Vec<[F; COLUMNS]>,
) -> Vec<PolynomialValues<F>> {
    let trace_row_vecs = trace_rows.into_iter().map(|row| row.to_vec()).collect_vec();
    let trace_col_vecs: Vec<Vec<F>> = transpose(&trace_row_vecs);
    trace_col_vecs
        .into_iter()
        .map(|column| PolynomialValues::new(column))
        .collect()
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