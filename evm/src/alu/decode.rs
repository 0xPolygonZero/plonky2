use plonky2::field::extension_field::Extendable;
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::alu::add;
use crate::alu::columns;
use crate::alu::sub;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

pub fn generate<F: RichField>(lv: &mut [F; columns::NUM_ALU_COLUMNS]) {
    if lv[columns::IS_ADD].is_one() {
        add::generate(lv);
    } else if lv[columns::IS_SUB].is_one() {
        sub::generate(lv);
    } else {
        todo!("the requested operation has not yet been implemented");
    }

    /*
    else if lv[IS_DIV].is_one() {
        generate_division(lv);
    } else if lv[IS_AND].is_one() {
        generate_bitop(lv, IS_AND);
    } else if lv[IS_IOR].is_one() {
        generate_bitop(lv, IS_IOR);
    } else if lv[IS_XOR].is_one() {
        generate_bitop(lv, IS_XOR);
    } else if lv[IS_ANDNOT].is_one() {
        generate_bitop(lv, IS_ANDNOT);
    } else if lv[IS_ROTATE_LEFT].is_one() {
        generate_rotate_shift(lv, IS_ROTATE_LEFT);
    } else if lv[IS_ROTATE_RIGHT].is_one() {
        generate_rotate_shift(lv, IS_ROTATE_RIGHT);
    } else if lv[IS_SHIFT_LEFT].is_one() {
        generate_rotate_shift(lv, IS_SHIFT_LEFT);
    } else if lv[IS_SHIFT_RIGHT].is_one() {
        generate_rotate_shift(lv, IS_SHIFT_RIGHT);
    }
     */
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; columns::NUM_ALU_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // FIXME: Not sure this is needed; should be enforced by the CPU?
    /*
    // Check that the operation flag values are binary.
    for col in ALL_OPERATIONS {
        let val = lv[col];
        yield_constr.constraint(val * val - val);
    }
     */

    add::eval_packed_generic(lv, yield_constr);
    sub::eval_packed_generic(lv, yield_constr);
    /*
    eval_division(lv, yield_constr);
    eval_bitop(lv, yield_constr);
    eval_rotate_left(lv, yield_constr);
    eval_rotate_right(lv, yield_constr);
    eval_shift_left(lv, yield_constr);
    eval_shift_right(lv, yield_constr);
    */
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; columns::NUM_ALU_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    // FIXME: Not sure this is needed; should be enforced by the CPU?
    /*
    // Check that the operation flag values are binary.
    for col in ALL_OPERATIONS {
        let val = lv[col];
        let constraint = builder.mul_sub_extension(val, val, val);
        yield_constr.constraint(builder, constraint);
    }
     */

    add::eval_ext_circuit(builder, lv, yield_constr);
    sub::eval_ext_circuit(builder, lv, yield_constr);
    /*
    eval_division_circuit(builder, lv, yield_constr);
    eval_bitop_circuit(builder, lv, yield_constr);
    eval_rotate_left_circuit(builder, lv, yield_constr);
    eval_rotate_right_circuit(builder, lv, yield_constr);
    eval_shift_left_circuit(builder, lv, yield_constr);
    eval_shift_right_circuit(builder, lv, yield_constr);
    */
}
