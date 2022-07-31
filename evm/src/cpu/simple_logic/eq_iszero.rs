use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;

const LIMB_SIZE: usize = 16;

pub fn generate<F: RichField>(lv: &mut CpuColumnsView<F>) {
    let logic = lv.general.logic_mut();
    let eq_filter = lv.is_eq.to_canonical_u64();
    let iszero_filter = lv.is_iszero.to_canonical_u64();
    assert!(eq_filter <= 1);
    assert!(iszero_filter <= 1);
    assert!(eq_filter + iszero_filter <= 1);

    if eq_filter != 1 && iszero_filter != 1 {
        return;
    }

    let diffs = if eq_filter == 1 {
        logic
            .input0
            .into_iter()
            .zip(logic.input1)
            .map(|(in0, in1)| {
                assert_eq!(in0.to_canonical_u64() >> LIMB_SIZE, 0);
                assert_eq!(in1.to_canonical_u64() >> LIMB_SIZE, 0);
                let diff = in0 - in1;
                diff.square()
            })
            .sum()
    } else if iszero_filter == 1 {
        logic.input0.into_iter().sum()
    } else {
        panic!()
    };

    lv.simple_logic_diff = diffs;
    lv.simple_logic_diff_inv = diffs.try_inverse().unwrap_or(F::ZERO);

    logic.output[0] = F::from_bool(diffs == F::ZERO);
    for out_limb_ref in logic.output[1..].iter_mut() {
        *out_limb_ref = F::ZERO;
    }
}

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let logic = lv.general.logic();
    let eq_filter = lv.is_eq;
    let iszero_filter = lv.is_iszero;
    let eq_or_iszero_filter = eq_filter + iszero_filter;

    let ls_bit = logic.output[0];

    // Handle EQ and ISZERO. Most limbs of the output are 0, but the least-significant one is
    // either 0 or 1.
    yield_constr.constraint(eq_or_iszero_filter * ls_bit * (ls_bit - P::ONES));

    for &bit in &logic.output[1..] {
        yield_constr.constraint(eq_or_iszero_filter * bit);
    }

    // Check SIMPLE_LOGIC_DIFF
    let diffs = lv.simple_logic_diff;
    let diffs_inv = lv.simple_logic_diff_inv;
    {
        let input0_sum: P = logic.input0.into_iter().sum();
        yield_constr.constraint(iszero_filter * (diffs - input0_sum));

        let sum_squared_diffs: P = logic
            .input0
            .into_iter()
            .zip(logic.input1)
            .map(|(in0, in1)| (in0 - in1).square())
            .sum();
        yield_constr.constraint(eq_filter * (diffs - sum_squared_diffs));
    }

    // diffs != 0 => ls_bit == 0
    yield_constr.constraint(eq_or_iszero_filter * diffs * ls_bit);
    // ls_bit == 0 => diffs != 0 (we provide a diffs_inv)
    yield_constr.constraint(eq_or_iszero_filter * (diffs * diffs_inv + ls_bit - P::ONES));
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let logic = lv.general.logic();
    let eq_filter = lv.is_eq;
    let iszero_filter = lv.is_iszero;
    let eq_or_iszero_filter = builder.add_extension(eq_filter, iszero_filter);

    let ls_bit = logic.output[0];

    // Handle EQ and ISZERO. Most limbs of the output are 0, but the least-significant one is
    // either 0 or 1.
    {
        let constr = builder.mul_sub_extension(ls_bit, ls_bit, ls_bit);
        let constr = builder.mul_extension(eq_or_iszero_filter, constr);
        yield_constr.constraint(builder, constr);
    }

    for &bit in &logic.output[1..] {
        let constr = builder.mul_extension(eq_or_iszero_filter, bit);
        yield_constr.constraint(builder, constr);
    }

    // Check SIMPLE_LOGIC_DIFF
    let diffs = lv.simple_logic_diff;
    let diffs_inv = lv.simple_logic_diff_inv;
    {
        let input0_sum = builder.add_many_extension(logic.input0);
        {
            let constr = builder.sub_extension(diffs, input0_sum);
            let constr = builder.mul_extension(iszero_filter, constr);
            yield_constr.constraint(builder, constr);
        }

        let sum_squared_diffs = logic.input0.into_iter().zip(logic.input1).fold(
            builder.zero_extension(),
            |acc, (in0, in1)| {
                let diff = builder.sub_extension(in0, in1);
                builder.mul_add_extension(diff, diff, acc)
            },
        );
        {
            let constr = builder.sub_extension(diffs, sum_squared_diffs);
            let constr = builder.mul_extension(eq_filter, constr);
            yield_constr.constraint(builder, constr);
        }
    }

    {
        // diffs != 0 => ls_bit == 0
        let constr = builder.mul_extension(diffs, ls_bit);
        let constr = builder.mul_extension(eq_or_iszero_filter, constr);
        yield_constr.constraint(builder, constr);
    }
    {
        // ls_bit == 0 => diffs != 0 (we provide a diffs_inv)
        let constr = builder.mul_add_extension(diffs, diffs_inv, ls_bit);
        let constr = builder.mul_sub_extension(eq_or_iszero_filter, constr, eq_or_iszero_filter);
        yield_constr.constraint(builder, constr);
    }
}
