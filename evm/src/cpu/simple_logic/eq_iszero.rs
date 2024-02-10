use ethereum_types::U256;
use itertools::izip;
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;

use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::cpu::columns::CpuColumnsView;
use crate::cpu::stack::{self, EQ_STACK_BEHAVIOR, IS_ZERO_STACK_BEHAVIOR};

fn limbs(x: U256) -> [u32; 8] {
    let mut res = [0; 8];
    let x_u64: [u64; 4] = x.0;
    for i in 0..4 {
        res[2 * i] = x_u64[i] as u32;
        res[2 * i + 1] = (x_u64[i] >> 32) as u32;
    }
    res
}
/// Form `diff_pinv`.
/// Let `diff = val0 - val1`. Consider `x[i] = diff[i]^-1` if `diff[i] != 0` and 0 otherwise.
/// Then `diff @ x = num_unequal_limbs`, where `@` denotes the dot product. We set
/// `diff_pinv = num_unequal_limbs^-1 * x` if `num_unequal_limbs != 0` and 0 otherwise. We have
/// `diff @ diff_pinv = 1 - equal` as desired.
pub(crate) fn generate_pinv_diff<F: Field>(val0: U256, val1: U256, lv: &mut CpuColumnsView<F>) {
    let val0_limbs = limbs(val0).map(F::from_canonical_u32);
    let val1_limbs = limbs(val1).map(F::from_canonical_u32);

    let num_unequal_limbs = izip!(val0_limbs, val1_limbs)
        .map(|(limb0, limb1)| (limb0 != limb1) as usize)
        .sum();

    // Form `diff_pinv`.
    let logic = lv.general.logic_mut();
    let num_unequal_limbs_inv = F::from_canonical_usize(num_unequal_limbs)
        .try_inverse()
        .unwrap_or(F::ZERO);
    for (limb_pinv, limb0, limb1) in izip!(logic.diff_pinv.iter_mut(), val0_limbs, val1_limbs) {
        *limb_pinv = (limb0 - limb1).try_inverse().unwrap_or(F::ZERO) * num_unequal_limbs_inv;
    }
}

/// Evaluates the constraints for EQ and ISZERO.
pub(crate) fn eval_packed<P: PackedField>(
    lv: &CpuColumnsView<P>,
    nv: &CpuColumnsView<P>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let logic = lv.general.logic();
    let input0 = lv.mem_channels[0].value;
    let input1 = lv.mem_channels[1].value;
    let output = nv.mem_channels[0].value;

    // EQ (0x14) and ISZERO (0x15) are differentiated by their first opcode bit.
    let eq_filter = lv.op.eq_iszero * (P::ONES - lv.opcode_bits[0]);
    let iszero_filter = lv.op.eq_iszero * lv.opcode_bits[0];
    let eq_or_iszero_filter = lv.op.eq_iszero;

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

    // Stack constraints.
    stack::eval_packed_one(lv, nv, eq_filter, EQ_STACK_BEHAVIOR.unwrap(), yield_constr);
    stack::eval_packed_one(
        lv,
        nv,
        iszero_filter,
        IS_ZERO_STACK_BEHAVIOR.unwrap(),
        yield_constr,
    );
}

/// Circuit version of `eval_packed`.
/// Evaluates the constraints for EQ and ISZERO.
pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut plonky2::plonk::circuit_builder::CircuitBuilder<F, D>,
    lv: &CpuColumnsView<ExtensionTarget<D>>,
    nv: &CpuColumnsView<ExtensionTarget<D>>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let zero = builder.zero_extension();
    let one = builder.one_extension();

    let logic = lv.general.logic();
    let input0 = lv.mem_channels[0].value;
    let input1 = lv.mem_channels[1].value;
    let output = nv.mem_channels[0].value;

    // EQ (0x14) and ISZERO (0x15) are differentiated by their first opcode bit.
    let eq_filter = builder.mul_extension(lv.op.eq_iszero, lv.opcode_bits[0]);
    let eq_filter = builder.sub_extension(lv.op.eq_iszero, eq_filter);

    let iszero_filter = builder.mul_extension(lv.op.eq_iszero, lv.opcode_bits[0]);
    let eq_or_iszero_filter = lv.op.eq_iszero;

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

    // Stack constraints.
    stack::eval_ext_circuit_one(
        builder,
        lv,
        nv,
        eq_filter,
        EQ_STACK_BEHAVIOR.unwrap(),
        yield_constr,
    );
    stack::eval_ext_circuit_one(
        builder,
        lv,
        nv,
        iszero_filter,
        IS_ZERO_STACK_BEHAVIOR.unwrap(),
        yield_constr,
    );
}
