use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::Field;
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use starky::vars::StarkEvaluationTargets;
use starky::vars::StarkEvaluationVars;

use crate::arithmetic::addition::{eval_addition, eval_addition_recursively, generate_addition};
use crate::arithmetic::division::{eval_division, eval_division_recursively, generate_division};
use crate::arithmetic::multiplication::{
    eval_multiplication, eval_multiplication_recursively, generate_multiplication,
};
use crate::arithmetic::subtraction::{
    eval_subtraction, eval_subtraction_recursively, generate_subtraction,
};
use crate::column_layout::arithmetic::*;
use crate::column_layout::NUM_COLUMNS;
use crate::public_input_layout::NUM_PUBLIC_INPUTS;

mod addition;
mod division;
mod multiplication;
mod subtraction;

pub(crate) fn generate_arithmetic_unit<F: RichField>(values: &mut [F; NUM_COLUMNS]) {
    if values[IS_ADD].is_one() {
        generate_addition(values);
    } else if values[IS_SUB].is_one() {
        generate_subtraction(values);
    } else if values[IS_MUL].is_one() {
        generate_multiplication(values);
    } else if values[IS_DIV].is_one() {
        generate_division(values);
    }
}

pub(crate) fn eval_arithmetic_unit<F: Field, P: PackedField<Scalar = F>>(
    vars: StarkEvaluationVars<F, P, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let local_values = &vars.local_values;

    // Check that the operation flag values are binary.
    for col in [IS_ADD, IS_SUB, IS_MUL, IS_DIV] {
        let val = local_values[col];
        yield_constr.constraint_wrapping(val * val - val);
    }

    eval_addition(local_values, yield_constr);
    eval_subtraction(local_values, yield_constr);
    eval_multiplication(local_values, yield_constr);
    eval_division(local_values, yield_constr);
}

pub(crate) fn eval_arithmetic_unit_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    vars: StarkEvaluationTargets<D, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let local_values = &vars.local_values;

    // Check that the operation flag values are binary.
    for col in [IS_ADD, IS_SUB, IS_MUL, IS_DIV] {
        let val = local_values[col];
        let constraint = builder.mul_add_extension(val, val, val);
        yield_constr.constraint_wrapping(builder, constraint);
    }

    eval_addition_recursively(builder, local_values, yield_constr);
    eval_subtraction_recursively(builder, local_values, yield_constr);
    eval_multiplication_recursively(builder, local_values, yield_constr);
    eval_division_recursively(builder, local_values, yield_constr);
}
