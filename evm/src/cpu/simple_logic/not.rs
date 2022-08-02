use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;

const LIMB_SIZE: usize = 16;
const ALL_1_LIMB: u64 = (1 << LIMB_SIZE) - 1;

pub fn generate<F: RichField>(lv: &mut CpuColumnsView<F>) {
    let is_not_filter = lv.is_not.to_canonical_u64();
    if is_not_filter == 0 {
        return;
    }
    assert_eq!(is_not_filter, 1);

    let logic = lv.general.logic_mut();
    for (input, output_ref) in logic.input0.into_iter().zip(logic.output.iter_mut()) {
        let input = input.to_canonical_u64();
        assert_eq!(input >> LIMB_SIZE, 0);
        let output = input ^ ALL_1_LIMB;
        *output_ref = F::from_canonical_u64(output);
    }
}

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // This is simple: just do output = 0xffff - input.
    let logic = lv.general.logic();
    let cycle_filter = lv.is_cpu_cycle;
    let is_not_filter = lv.is_not;
    let filter = cycle_filter * is_not_filter;
    for (input, output) in logic.input0.into_iter().zip(logic.output) {
        yield_constr
            .constraint(filter * (output + input - P::Scalar::from_canonical_u64(ALL_1_LIMB)));
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let logic = lv.general.logic();
    let cycle_filter = lv.is_cpu_cycle;
    let is_not_filter = lv.is_not;
    let filter = builder.mul_extension(cycle_filter, is_not_filter);
    for (input, output) in logic.input0.into_iter().zip(logic.output) {
        let constr = builder.add_extension(output, input);
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
