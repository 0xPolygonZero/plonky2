use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::{Field, PrimeField64};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use starky::vars::StarkEvaluationTargets;
use starky::vars::StarkEvaluationVars;

use crate::alu::addition::{eval_addition, eval_addition_recursively, generate_addition};
use crate::alu::bitops::{eval_bitop, eval_bitop_recursively, generate_bitop};
use crate::alu::division::{eval_division, eval_division_recursively, generate_division};
use crate::alu::mul_add::{eval_mul_add, eval_mul_add_recursively, generate_mul_add};
use crate::alu::subtraction::{
    eval_subtraction, eval_subtraction_recursively, generate_subtraction,
};
use crate::public_input_layout::NUM_PUBLIC_INPUTS;
use crate::registers::alu::*;
use crate::registers::NUM_COLUMNS;

mod addition;
mod bitops;
mod canonical;
mod division;
mod mul_add;
mod subtraction;

// TODO: This probably belongs in a more easily accessible location.
const ALL_OPERATIONS: [usize; 8] = [
    IS_ADD,
    IS_SUB,
    IS_MUL_ADD,
    IS_DIV,
    IS_AND,
    IS_IOR,
    IS_XOR,
    IS_ANDNOT,
];

pub(crate) fn generate_alu<F: PrimeField64>(values: &mut [F; NUM_COLUMNS]) {
    if values[IS_ADD].is_one() {
        generate_addition(values);
    } else if values[IS_SUB].is_one() {
        generate_subtraction(values);
    } else if values[IS_MUL_ADD].is_one() {
        generate_mul_add(values);
    } else if values[IS_DIV].is_one() {
        generate_division(values);
    } else if values[IS_AND].is_one() {
        generate_bitop(values, IS_AND);
    } else if values[IS_IOR].is_one() {
        generate_bitop(values, IS_IOR);
    } else if values[IS_XOR].is_one() {
        generate_bitop(values, IS_XOR);
    } else if values[IS_ANDNOT].is_one() {
        generate_bitop(values, IS_ANDNOT);
    }
}

pub(crate) fn eval_alu<F: Field, P: PackedField<Scalar = F>>(
    vars: StarkEvaluationVars<F, P, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let local_values = &vars.local_values;

    // Check that the operation flag values are binary.
    for col in ALL_OPERATIONS {
        let val = local_values[col];
        yield_constr.constraint(val * val - val);
    }

    eval_addition(local_values, yield_constr);
    eval_subtraction(local_values, yield_constr);
    eval_mul_add(local_values, yield_constr);
    eval_division(local_values, yield_constr);
    eval_bitop(local_values, yield_constr);
}

pub(crate) fn eval_alu_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    vars: StarkEvaluationTargets<D, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let local_values = &vars.local_values;

    // Check that the operation flag values are binary.
    for col in ALL_OPERATIONS {
        let val = local_values[col];
        let constraint = builder.mul_sub_extension(val, val, val);
        yield_constr.constraint(builder, constraint);
    }

    eval_addition_recursively(builder, local_values, yield_constr);
    eval_subtraction_recursively(builder, local_values, yield_constr);
    eval_mul_add_recursively(builder, local_values, yield_constr);
    eval_division_recursively(builder, local_values, yield_constr);
    eval_bitop_recursively(builder, local_values, yield_constr);
}
