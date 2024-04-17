pub(crate) mod eq_iszero;
mod not;

use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;

/// Evaluates constraints for NOT, EQ and ISZERO.
pub(crate) fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    not::eval_packed(lv, nv, yield_constr);
    eq_iszero::eval_packed(lv, nv, yield_constr);
}

/// Circuit version of `eval_packed`.
/// Evaluates constraints for NOT, EQ and ISZERO.
pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    not::eval_ext_circuit(builder, lv, nv, yield_constr);
    eq_iszero::eval_ext_circuit(builder, lv, nv, yield_constr);
}
