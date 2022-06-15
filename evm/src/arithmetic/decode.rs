use plonky2::field::extension_field::Extendable;
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::arithmetic::add;
use crate::arithmetic::columns;
use crate::arithmetic::mul;
use crate::arithmetic::sub;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

pub fn generate<F: RichField>(lv: &mut [F; columns::NUM_ARITH_COLUMNS]) {
    if lv[columns::IS_ADD].is_one() {
        add::generate(lv);
    } else if lv[columns::IS_SUB].is_one() {
        sub::generate(lv);
    } else if lv[columns::IS_MUL].is_one() {
        mul::generate(lv);
    } else {
        todo!("the requested operation has not yet been implemented");
    }
}

pub fn eval_packed_generic<P: PackedField>(
    lv: &[P; columns::NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // FIXME: Not sure this is needed; should be enforced by the CPU?
    // And if it is needed, shouldn't we also require that only one
    // value is non-zero?
    /*
    // Check that the operation flag values are binary.
    for col in ALL_OPERATIONS {
        let val = lv[col];
        yield_constr.constraint(val * val - val);
    }
     */

    add::eval_packed_generic(lv, yield_constr);
    sub::eval_packed_generic(lv, yield_constr);
    mul::eval_packed_generic(lv, yield_constr);
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; columns::NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    // FIXME: Not sure this is needed; should be enforced by the CPU?
    // And if it is needed, shouldn't we also require that only one
    // value is non-zero?
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
    mul::eval_ext_circuit(builder, lv, yield_constr);
}
