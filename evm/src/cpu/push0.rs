use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::membus::NUM_GP_CHANNELS;

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let filter = lv.op.push0;
    let push_value = lv.mem_channels[NUM_GP_CHANNELS - 1].value;
    for limb in push_value {
        yield_constr.constraint(filter * limb);
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let filter = lv.op.push0;
    let push_value = lv.mem_channels[NUM_GP_CHANNELS - 1].value;
    for limb in push_value {
        let constr = builder.mul_extension(filter, limb);
        yield_constr.constraint(builder, constr);
    }
}
