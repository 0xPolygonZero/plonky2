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
use plonky2::util::lookups::permuted_cols;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use starky::vars::StarkEvaluationTargets;
use starky::vars::StarkEvaluationVars;

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

pub(crate) fn eval_lookups_recursively<F: RichField + Extendable<D>, const D: usize>(
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
