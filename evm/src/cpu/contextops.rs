use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::membus::NUM_GP_CHANNELS;

fn eval_packed_get<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let filter = lv.op.get_context;
    let push_channel = lv.mem_channels[NUM_GP_CHANNELS - 1];
    yield_constr.constraint(filter * (push_channel.value[0] - lv.context));
    for &limb in &push_channel.value[1..] {
        yield_constr.constraint(filter * limb);
    }
}

fn eval_ext_circuit_get<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let filter = lv.op.get_context;
    let push_channel = lv.mem_channels[NUM_GP_CHANNELS - 1];
    {
        let diff = builder.sub_extension(push_channel.value[0], lv.context);
        let constr = builder.mul_extension(filter, diff);
        yield_constr.constraint(builder, constr);
    }
    for &limb in &push_channel.value[1..] {
        let constr = builder.mul_extension(filter, limb);
        yield_constr.constraint(builder, constr);
    }
}

fn eval_packed_set<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let filter = lv.op.set_context;
    let pop_channel = lv.mem_channels[0];
    yield_constr.constraint_transition(filter * (pop_channel.value[0] - nv.context));
}

fn eval_ext_circuit_set<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let filter = lv.op.set_context;
    let pop_channel = lv.mem_channels[0];

    let diff = builder.sub_extension(pop_channel.value[0], nv.context);
    let constr = builder.mul_extension(filter, diff);
    yield_constr.constraint_transition(builder, constr);
}

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    eval_packed_get(lv, yield_constr);
    eval_packed_set(lv, nv, yield_constr);
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    eval_ext_circuit_get(builder, lv, yield_constr);
    eval_ext_circuit_set(builder, lv, nv, yield_constr);
}
