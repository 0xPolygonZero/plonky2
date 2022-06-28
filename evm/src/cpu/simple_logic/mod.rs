mod eq_iszero;
mod not;

use std::borrow::{Borrow, BorrowMut};

use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::{CpuColumnsView, NUM_CPU_COLUMNS};

pub fn generate<F: RichField>(lv: &mut [F; NUM_CPU_COLUMNS]) {
    let lv: &mut CpuColumnsView<_> = lv.borrow_mut();
    let cycle_filter = lv.is_cpu_cycle.to_canonical_u64();
    if cycle_filter == 0 {
        return;
    }
    assert_eq!(cycle_filter, 1);

    not::generate(lv);
    eq_iszero::generate(lv);
}

pub fn eval_packed<P: PackedField>(
    lv: &[P; NUM_CPU_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let lv: &CpuColumnsView<_> = lv.borrow();
    not::eval_packed(lv, yield_constr);
    eq_iszero::eval_packed(lv, yield_constr);
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_CPU_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let lv: &CpuColumnsView<_> = lv.borrow();
    not::eval_ext_circuit(builder, lv, yield_constr);
    eq_iszero::eval_ext_circuit(builder, lv, yield_constr);
}
