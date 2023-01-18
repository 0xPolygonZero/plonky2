use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::membus::NUM_GP_CHANNELS;

const LIMB_SIZE: usize = 32;
const ALL_1_LIMB: u64 = (1 << LIMB_SIZE) - 1;

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // This is simple: just do output = 0xffffffff - input.
    let input = lv.mem_channels[0].value;
    let output = lv.mem_channels[NUM_GP_CHANNELS - 1].value;
    let cycle_filter = lv.is_cpu_cycle;
    let is_not_filter = lv.op.not;
    let filter = cycle_filter * is_not_filter;
    for (input_limb, output_limb) in input.into_iter().zip(output) {
        yield_constr.constraint(
            filter * (output_limb + input_limb - P::Scalar::from_canonical_u64(ALL_1_LIMB)),
        );
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let input = lv.mem_channels[0].value;
    let output = lv.mem_channels[NUM_GP_CHANNELS - 1].value;
    let cycle_filter = lv.is_cpu_cycle;
    let is_not_filter = lv.op.not;
    let filter = builder.mul_extension(cycle_filter, is_not_filter);
    for (input_limb, output_limb) in input.into_iter().zip(output) {
        let constr = builder.add_extension(output_limb, input_limb);
        let constr = builder.arithmetic_extension(
            F::ONE,
            -F::from_canonical_u64(ALL_1_LIMB),
            filter,
            constr,
            filter,
        );
        yield_constr.constraint(builder, constr);
    }
}
