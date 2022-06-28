use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::registers;

const LIMB_SIZE: usize = 16;
const ALL_1_LIMB: u64 = (1 << LIMB_SIZE) - 1;

pub fn generate<F: RichField>(lv: &mut [F; registers::NUM_CPU_COLUMNS]) {
    let is_not_filter = lv[registers::IS_NOT].to_canonical_u64();
    if is_not_filter == 0 {
        return;
    }
    assert_eq!(is_not_filter, 1);

    for (input_col, output_col) in registers::LOGIC_INPUT0.zip(registers::LOGIC_OUTPUT) {
        let input = lv[input_col].to_canonical_u64();
        assert_eq!(input >> LIMB_SIZE, 0);
        let output = input ^ ALL_1_LIMB;
        lv[output_col] = F::from_canonical_u64(output);
    }
}

pub fn eval_packed<P: PackedField>(
    lv: &[P; registers::NUM_CPU_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // This is simple: just do output = 0xffff - input.
    let cycle_filter = lv[registers::IS_CPU_CYCLE];
    let is_not_filter = lv[registers::IS_NOT];
    let filter = cycle_filter * is_not_filter;
    for (input_col, output_col) in registers::LOGIC_INPUT0.zip(registers::LOGIC_OUTPUT) {
        let input = lv[input_col];
        let output = lv[output_col];
        yield_constr
            .constraint(filter * (output + input - P::Scalar::from_canonical_u64(ALL_1_LIMB)));
    }
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; registers::NUM_CPU_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let cycle_filter = lv[registers::IS_CPU_CYCLE];
    let is_not_filter = lv[registers::IS_NOT];
    let filter = builder.mul_extension(cycle_filter, is_not_filter);
    for (input_col, output_col) in registers::LOGIC_INPUT0.zip(registers::LOGIC_OUTPUT) {
        let input = lv[input_col];
        let output = lv[output_col];
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
