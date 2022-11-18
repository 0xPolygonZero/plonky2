use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::{Field, PrimeField64};
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use starky::vars::{StarkEvaluationTargets, StarkEvaluationVars};

use crate::alu::addition::{eval_addition, eval_addition_circuit, generate_addition};
use crate::alu::bitops::{eval_bitop, eval_bitop_circuit, generate_bitop};
use crate::alu::division::{eval_division, eval_division_circuit, generate_division};
use crate::alu::mul_add::{eval_mul_add, eval_mul_add_circuit, generate_mul_add};
use crate::alu::rotate_shift::{
    eval_rotate_left, eval_rotate_left_circuit, eval_rotate_right, eval_rotate_right_circuit,
    eval_shift_left, eval_shift_left_circuit, eval_shift_right, eval_shift_right_circuit,
    generate_rotate_shift,
};
use crate::alu::subtraction::{eval_subtraction, eval_subtraction_circuit, generate_subtraction};
use crate::public_input_layout::NUM_PUBLIC_INPUTS;
use crate::registers::alu::*;
use crate::registers::NUM_COLUMNS;

mod addition;
mod bitops;
mod canonical;
mod division;
mod mul_add;
mod rotate_shift;
mod subtraction;

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
    } else if values[IS_ROTATE_LEFT].is_one() {
        generate_rotate_shift(values, IS_ROTATE_LEFT);
    } else if values[IS_ROTATE_RIGHT].is_one() {
        generate_rotate_shift(values, IS_ROTATE_RIGHT);
    } else if values[IS_SHIFT_LEFT].is_one() {
        generate_rotate_shift(values, IS_SHIFT_LEFT);
    } else if values[IS_SHIFT_RIGHT].is_one() {
        generate_rotate_shift(values, IS_SHIFT_RIGHT);
    } else {
        //todo!("the requested operation has not yet been implemented");
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
    eval_rotate_left(local_values, yield_constr);
    eval_rotate_right(local_values, yield_constr);
    eval_shift_left(local_values, yield_constr);
    eval_shift_right(local_values, yield_constr);
}

pub(crate) fn eval_alu_circuit<F: RichField + Extendable<D>, const D: usize>(
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

    eval_addition_circuit(builder, local_values, yield_constr);
    eval_subtraction_circuit(builder, local_values, yield_constr);
    eval_mul_add_circuit(builder, local_values, yield_constr);
    eval_division_circuit(builder, local_values, yield_constr);
    eval_bitop_circuit(builder, local_values, yield_constr);
    eval_rotate_left_circuit(builder, local_values, yield_constr);
    eval_rotate_right_circuit(builder, local_values, yield_constr);
    eval_shift_left_circuit(builder, local_values, yield_constr);
    eval_shift_right_circuit(builder, local_values, yield_constr);
}
