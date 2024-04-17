use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;

/// Evaluates constraints to check that we are storing the correct PC.
pub(crate) fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // `PUSH0`'s opcode is odd, while `PC`'s opcode is even.
    let filter = lv.op.pc_push0 * (P::ONES - lv.opcode_bits[0]);
    let new_stack_top = nv.mem_channels[0].value;
    yield_constr.constraint(filter * (new_stack_top[0] - lv.program_counter));
    for &limb in &new_stack_top[1..] {
        yield_constr.constraint(filter * limb);
    }
}

/// Circuit version if `eval_packed`.
/// Evaluates constraints to check that we are storing the correct PC.
pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    // `PUSH0`'s opcode is odd, while `PC`'s opcode is even.
    let one = builder.one_extension();
    let mut filter = builder.sub_extension(one, lv.opcode_bits[0]);
    filter = builder.mul_extension(lv.op.pc_push0, filter);
    let new_stack_top = nv.mem_channels[0].value;
    {
        let diff = builder.sub_extension(new_stack_top[0], lv.program_counter);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    for &limb in &new_stack_top[1..] {
        let constr = builder.mul_extension(filter, limb);
        yield_constr.constraint(builder, constr);
    }
}
