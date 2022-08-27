use itertools::izip;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;

pub fn generate<F: RichField>(lv: &mut CpuColumnsView<F>) {
    let input0 = lv.mem_channels[0].value;

    let eq_filter = lv.is_eq.to_canonical_u64();
    let iszero_filter = lv.is_iszero.to_canonical_u64();
    assert!(eq_filter <= 1);
    assert!(iszero_filter <= 1);
    assert!(eq_filter + iszero_filter <= 1);

    if eq_filter + iszero_filter == 0 {
        return;
    }

    let input1 = &mut lv.mem_channels[1].value;
    if iszero_filter != 0 {
        for limb in input1.iter_mut() {
            *limb = F::ZERO;
        }
    }

    let input1 = lv.mem_channels[1].value;
    let num_unequal_limbs = izip!(input0, input1)
        .map(|(limb0, limb1)| (limb0 != limb1) as usize)
        .sum();
    let equal = num_unequal_limbs == 0;

    let output = &mut lv.mem_channels[2].value;
    output[0] = F::from_bool(equal);
    for limb in &mut output[1..] {
        *limb = F::ZERO;
    }

    // Form `diff_pinv`.
    // Let `diff = input0 - input1`. Consider `x[i] = diff[i]^-1` if `diff[i] != 0` and 0 otherwise.
    // Then `diff @ x = num_unequal_limbs`, where `@` denotes the dot product. We set
    // `diff_pinv = num_unequal_limbs^-1 * x` if `num_unequal_limbs != 0` and 0 otherwise. We have
    // `diff @ diff_pinv = 1 - equal` as desired.
    let logic = lv.general.logic_mut();
    let num_unequal_limbs_inv = F::from_canonical_usize(num_unequal_limbs)
        .try_inverse()
        .unwrap_or(F::ZERO);
    for (limb_pinv, limb0, limb1) in izip!(logic.diff_pinv.iter_mut(), input0, input1) {
        *limb_pinv = (limb0 - limb1).try_inverse().unwrap_or(F::ZERO) * num_unequal_limbs_inv;
    }
}

pub fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let logic = lv.general.logic();
    let input0 = lv.mem_channels[0].value;
    let input1 = lv.mem_channels[1].value;
    let output = lv.mem_channels[2].value;

    let eq_filter = lv.is_eq;
    let iszero_filter = lv.is_iszero;
    let eq_or_iszero_filter = eq_filter + iszero_filter;

    let equal = output[0];
    let unequal = P::ONES - equal;

    // Handle `EQ` and `ISZERO`. Most limbs of the output are 0, but the least-significant one is
    // either 0 or 1.
    yield_constr.constraint(eq_or_iszero_filter * equal * unequal);
    for &limb in &output[1..] {
        yield_constr.constraint(eq_or_iszero_filter * limb);
    }

    // If `ISZERO`, constrain input1 to be zero, effectively implementing ISZERO(x) as EQ(x, 0).
    for limb in input1 {
        yield_constr.constraint(iszero_filter * limb);
    }

    // `equal` implies `input0[i] == input1[i]` for all `i`.
    for (limb0, limb1) in izip!(input0, input1) {
        let diff = limb0 - limb1;
        yield_constr.constraint(eq_or_iszero_filter * equal * diff);
    }

    // `input0[i] == input1[i]` for all `i` implies `equal`.
    // If `unequal`, find `diff_pinv` such that `(input0 - input1) @ diff_pinv == 1`, where `@`
    // denotes the dot product (there will be many such `diff_pinv`). This can only be done if
    // `input0 != input1`.
    let dot: P = izip!(input0, input1, logic.diff_pinv)
        .map(|(limb0, limb1, diff_pinv_el)| (limb0 - limb1) * diff_pinv_el)
        .sum();
    yield_constr.constraint(eq_or_iszero_filter * (dot - unequal));
}

pub fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let zero = builder.zero_extension();
    let one = builder.one_extension();

    let logic = lv.general.logic();
    let input0 = lv.mem_channels[0].value;
    let input1 = lv.mem_channels[1].value;
    let output = lv.mem_channels[2].value;

    let eq_filter = lv.is_eq;
    let iszero_filter = lv.is_iszero;
    let eq_or_iszero_filter = builder.add_extension(eq_filter, iszero_filter);

    let equal = output[0];
    let unequal = builder.sub_extension(one, equal);

    // Handle `EQ` and `ISZERO`. Most limbs of the output are 0, but the least-significant one is
    // either 0 or 1.
    {
        let constr = builder.mul_extension(equal, unequal);
        let constr = builder.mul_extension(eq_or_iszero_filter, constr);
        yield_constr.constraint(builder, constr);
    }
    for &limb in &output[1..] {
        let constr = builder.mul_extension(eq_or_iszero_filter, limb);
        yield_constr.constraint(builder, constr);
    }

    // If `ISZERO`, constrain input1 to be zero, effectively implementing ISZERO(x) as EQ(x, 0).
    for limb in input1 {
        let constr = builder.mul_extension(iszero_filter, limb);
        yield_constr.constraint(builder, constr);
    }

    // `equal` implies `input0[i] == input1[i]` for all `i`.
    for (limb0, limb1) in izip!(input0, input1) {
        let diff = builder.sub_extension(limb0, limb1);
        let constr = builder.mul_extension(equal, diff);
        let constr = builder.mul_extension(eq_or_iszero_filter, constr);
        yield_constr.constraint(builder, constr);
    }

    // `input0[i] == input1[i]` for all `i` implies `equal`.
    // If `unequal`, find `diff_pinv` such that `(input0 - input1) @ diff_pinv == 1`, where `@`
    // denotes the dot product (there will be many such `diff_pinv`). This can only be done if
    // `input0 != input1`.
    {
        let dot: ExtensionTarget<D> = izip!(input0, input1, logic.diff_pinv).fold(
            zero,
            |cumul, (limb0, limb1, diff_pinv_el)| {
                let diff = builder.sub_extension(limb0, limb1);
                builder.mul_add_extension(diff, diff_pinv_el, cumul)
            },
        );
        let constr = builder.sub_extension(dot, unequal);
        let constr = builder.mul_extension(eq_or_iszero_filter, constr);
        yield_constr.constraint(builder, constr);
    }
}
