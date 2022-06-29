use std::ops::Add;

use itertools::Itertools;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::arithmetic::add;
use crate::arithmetic::columns;
use crate::arithmetic::mul;
use crate::arithmetic::sub;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

pub fn generate<F: RichField>(lv: &mut [F; columns::NUM_ARITH_COLUMNS]) {
    // Check that at most one operation column is "one" and that the
    // rest are "zero".
    assert_eq!(
        columns::ALL_OPERATIONS
            .iter()
            .map(|&c| {
                if lv[c] == F::ONE {
                    Ok(1u64)
                } else if lv[c] == F::ZERO {
                    Ok(0u64)
                } else {
                    Err("column was not 0 nor 1")
                }
            })
            .fold_ok(0u64, Add::add),
        Ok(1)
    );

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
    add::eval_packed_generic(lv, yield_constr);
    sub::eval_packed_generic(lv, yield_constr);
    mul::eval_packed_generic(lv, yield_constr);
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; columns::NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    add::eval_ext_circuit(builder, lv, yield_constr);
    sub::eval_ext_circuit(builder, lv, yield_constr);
    mul::eval_ext_circuit(builder, lv, yield_constr);
}
