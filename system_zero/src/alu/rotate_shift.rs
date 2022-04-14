use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::{Field, PrimeField64};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

use crate::alu::bitops::constrain_all_to_bits;
use crate::registers::alu::*;
use crate::registers::NUM_COLUMNS;

/// ROTATE LEFT
///
/// The input is
///
/// - a 64-bit integer X to be rotated, given as high and low 32-bit words X_lo
///   and X_hi.
/// - a 32-bit integer D giving the number of bits to rotate
/// - D mod 32 and D mod 64
/// - two 64-bit integers, Y_lo and Y_hi, with Y_lo being the high and low 32-bit
///   words of the value  2^{D'} * X_lo where D' = D (mod 32); similarly for Y_hi
/// - two auxiliary values, one each for Y_lo and Y_hi, used to prove that Y_lo and
///   Y_hi are valid Goldilocks elements.

pub(crate) fn generate_rotate_shift<F: PrimeField64>(values: &mut [F; NUM_COLUMNS], op: usize) {
    // input_{lo,hi} are the 32-bit lo and hi words of the input
    let input_lo = values[COL_ROTATE_SHIFT_INPUT_LO].to_canonical_u64();
    let input_hi = values[COL_ROTATE_SHIFT_INPUT_HI].to_canonical_u64();

    // Given the 6 bits delta_bits[0..5], bits 0..4 represent
    // delta_mod32 for left ops and (32 - delta_mod32) % 32 for right
    // ops, and delta_bits[5] represents whether delta > 32.

    // delta is the displacement amount. EXP_BITS holds the 5 bits of
    // either delta mod 32 (for left ops) or (32 - delta mod 32) mod 32
    // for right ops.
    let exp_bits = COL_ROTATE_SHIFT_EXP_BITS.map(|r| values[r].to_canonical_u64());

    let is_right_op = op == IS_ROTATE_RIGHT || op == IS_SHIFT_RIGHT || op == IS_ARITH_SHIFT_RIGHT;
    let exp: u64 = [0, 1, 2, 3, 4].map(|i| exp_bits[i] << i).into_iter().sum();
    let delta_mod32 = if is_right_op { (32u64 - exp) % 32 } else { exp };
    let top_bit = values[COL_ROTATE_SHIFT_DELTA_DIV32].to_canonical_u64();
    let delta = (top_bit << 5) + delta_mod32;

    // helper values
    let pow_exp_aux_0 = (exp_bits[0] + 1) * (3 * exp_bits[1] + 1);
    let pow_exp_aux_1 = (15 * exp_bits[2] + 1) * (255 * exp_bits[3] + 1);
    let pow_exp_aux_2 = pow_exp_aux_0 * pow_exp_aux_1;
    let pow_exp = pow_exp_aux_2 * (65535 * exp_bits[4] + 1);
    values[COL_ROTATE_SHIFT_POW_EXP_AUX_0] = F::from_canonical_u64(pow_exp_aux_0);
    values[COL_ROTATE_SHIFT_POW_EXP_AUX_1] = F::from_canonical_u64(pow_exp_aux_1);
    values[COL_ROTATE_SHIFT_POW_EXP_AUX_2] = F::from_canonical_u64(pow_exp_aux_2);
    values[COL_ROTATE_SHIFT_POW_EXP] = F::from_canonical_u64(pow_exp);

    let shifted_lo = input_lo << exp;
    let shifted_lo_0 = shifted_lo as u32;
    let shifted_lo_1 = (shifted_lo >> 32) as u32;
    values[COL_ROTATE_SHIFT_DISPLACED_INPUT_LO_0] = F::from_canonical_u32(shifted_lo_0);
    values[COL_ROTATE_SHIFT_DISPLACED_INPUT_LO_1] = F::from_canonical_u32(shifted_lo_1);
    let shifted_hi = input_hi << exp;
    let shifted_hi_0 = shifted_hi as u32;
    let shifted_hi_1 = (shifted_hi >> 32) as u32;
    values[COL_ROTATE_SHIFT_DISPLACED_INPUT_HI_0] = F::from_canonical_u32(shifted_hi_0);
    values[COL_ROTATE_SHIFT_DISPLACED_INPUT_HI_1] = F::from_canonical_u32(shifted_hi_1);

    if shifted_lo_1 != u32::MAX {
        let diff = F::from_canonical_u32(u32::MAX - shifted_lo_1);
        let inv = diff.inverse();
        values[COL_ROTATE_SHIFT_DISPLACED_INPUT_LO_AUX_0] = inv;
        values[COL_ROTATE_SHIFT_DISPLACED_INPUT_LO_AUX_1] = diff * inv;
    } else {
        // shifted_lo_0 must be zero, so this is unused.
        values[COL_ROTATE_SHIFT_DISPLACED_INPUT_LO_AUX_0] = F::ZERO;
        values[COL_ROTATE_SHIFT_DISPLACED_INPUT_LO_AUX_1] = F::ZERO;
    }
    if shifted_hi_1 != u32::MAX {
        let diff = F::from_canonical_u32(u32::MAX - shifted_hi_1);
        let inv = diff.inverse();
        values[COL_ROTATE_SHIFT_DISPLACED_INPUT_HI_AUX_0] = inv;
        values[COL_ROTATE_SHIFT_DISPLACED_INPUT_HI_AUX_1] = diff * inv;
    } else {
        // shifted_hi_0 must be zero, so this is unused.
        values[COL_ROTATE_SHIFT_DISPLACED_INPUT_HI_AUX_0] = F::ZERO;
        values[COL_ROTATE_SHIFT_DISPLACED_INPUT_HI_AUX_1] = F::ZERO;
    }

    // the input and output as u64s
    let input = (input_hi << 32) | input_lo;
    let delta = delta as u32;
    let output = match op {
        IS_ROTATE_LEFT => input.rotate_left(delta),
        IS_ROTATE_RIGHT => input.rotate_right(delta),
        IS_SHIFT_LEFT => input << delta,
        IS_SHIFT_RIGHT => input >> delta,
        IS_ARITH_SHIFT_RIGHT => (input as i64 >> delta) as u64,
        _ => panic!("unrecognized rotate/shift instruction code"),
    };

    // Output in base 2^16.
    values[COL_ROTATE_SHIFT_OUTPUT_0] = F::from_canonical_u32(output as u32);
    values[COL_ROTATE_SHIFT_OUTPUT_1] = F::from_canonical_u32((output >> 32) as u32);
}

pub(crate) fn eval_rotate_shift<F: Field, P: PackedField<Scalar = F>>(
    lv: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
    filter: P,
) -> (P, P, P, P, P, P, P) {
    // Input
    let input_lo = lv[COL_ROTATE_SHIFT_INPUT_LO];
    let input_hi = lv[COL_ROTATE_SHIFT_INPUT_HI];

    // Delta is the shift/rotate displacement; exp is delta mod 32 or
    // (32 - delta mod 32) mod 32, depending on whether the operation
    // direction is left or right.
    let exp_bits = COL_ROTATE_SHIFT_EXP_BITS.map(|r| lv[r]);
    let top_bit = lv[COL_ROTATE_SHIFT_DELTA_DIV32];

    let pow_exp_aux_0 = lv[COL_ROTATE_SHIFT_POW_EXP_AUX_0];
    let pow_exp_aux_1 = lv[COL_ROTATE_SHIFT_POW_EXP_AUX_1];
    let pow_exp_aux_2 = lv[COL_ROTATE_SHIFT_POW_EXP_AUX_2];
    let pow_exp = lv[COL_ROTATE_SHIFT_POW_EXP];

    let shifted_lo_0 = lv[COL_ROTATE_SHIFT_DISPLACED_INPUT_LO_0];
    let shifted_lo_1 = lv[COL_ROTATE_SHIFT_DISPLACED_INPUT_LO_1];
    let shifted_hi_0 = lv[COL_ROTATE_SHIFT_DISPLACED_INPUT_HI_0];
    let shifted_hi_1 = lv[COL_ROTATE_SHIFT_DISPLACED_INPUT_HI_1];
    let shifted_lo_aux_0 = lv[COL_ROTATE_SHIFT_DISPLACED_INPUT_LO_AUX_0];
    let shifted_lo_aux_1 = lv[COL_ROTATE_SHIFT_DISPLACED_INPUT_LO_AUX_1];
    let shifted_hi_aux_0 = lv[COL_ROTATE_SHIFT_DISPLACED_INPUT_HI_AUX_0];
    let shifted_hi_aux_1 = lv[COL_ROTATE_SHIFT_DISPLACED_INPUT_HI_AUX_1];

    // Output
    let output_lo = lv[COL_ROTATE_SHIFT_OUTPUT_0];
    let output_hi = lv[COL_ROTATE_SHIFT_OUTPUT_1];

    // Check that every "bit" of exp_bits and top_bit is 0 or 1
    exp_bits.map(|b| yield_constr.constraint(filter * (b * b - b)));
    yield_constr.constraint(filter * (top_bit * top_bit - top_bit));

    // Check that pow_exp = 2^exp, where exp is formed from the bits
    // exp_bits[0..4].
    //
    // 2^exp = \prod_i=0^4 (2^(2^i) if exp_bits[i] = 1 else 1)
    //       = \prod_i=0^4 ((2^(2^i) - 1) * exp_bits[i] + 1)
    //       = pow_exp
    //
    // on the conditions that:
    //
    // pow_exp_aux_0 = \prod_i=0^1 ((2^i - 1) * exp_bits[i] + 1)
    // pow_exp_aux_1 = \prod_i=2^3 ((2^i - 1) * exp_bits[i] + 1)
    // pow_exp_aux_2 = pow_exp_aux_0 * pow_exp_aux_1
    // pow_exp_mod32 = pow_exp_aux_2 * ((2^(2^4) - 1) * exp_bits[4] + 1)

    let one = P::ONES;
    // c[i-1] = 2^(2^i) - 1
    let c = [1, 2, 3, 4].map(|i| P::from(F::from_canonical_u64(1u64 << (1u32 << i))) - P::ONES);

    let constr1 = (exp_bits[0] + one) * (c[0] * exp_bits[1] + one);
    yield_constr.constraint(filter * (constr1 - pow_exp_aux_0));
    let constr2 = (c[1] * exp_bits[2] + one) * (c[2] * exp_bits[3] + one);
    yield_constr.constraint(filter * (constr2 - pow_exp_aux_1));
    let constr3 = pow_exp_aux_0 * pow_exp_aux_1;
    yield_constr.constraint(filter * (constr3 - pow_exp_aux_2));
    let constr4 = pow_exp_aux_2 * (c[3] * exp_bits[4] + one);
    yield_constr.constraint(filter * (constr4 - pow_exp));

    // An invalid shifted_lo (or _hi) can be too big to fit in
    // Goldilocks field; e.g. if both _0 and _1 parts are 2^32-1, then
    // shifted_lo = 2^32 - 1 + 2^32 (2^32 - 1) = 2^64 - 1 which
    // overflows in GoldilocksField. Hence we check that
    // shifted_{lo,hi} are valid Goldilocks elements following
    // https://hackmd.io/NC-yRmmtRQSvToTHb96e8Q#Checking-element-validity
    // The idea is check that a value v = (v_lo, v_hi) (32-bit words)
    // satisfies the condition (v_lo == 0 OR v_hi != 2^32-1), which
    // uses the structure of Goldilocks to check that v has the right
    // form. The formula is:
    //   v_lo * (one - aux * (u32_max - v_hi)) == 0
    // where aux = (m32_max - v_hi)^-1 if it exists.

    // u32_max = 2^32 - 1
    let u32_max = P::from(F::from_canonical_u32(u32::MAX));

    let constr1 = shifted_lo_aux_0 * (u32_max - shifted_lo_1);
    yield_constr.constraint(filter * (constr1 - shifted_lo_aux_1));
    let constr2 = shifted_hi_aux_0 * (u32_max - shifted_hi_1);
    yield_constr.constraint(filter * (constr2 - shifted_hi_aux_1));

    let shifted_lo_is_valid = shifted_lo_0 * (one - shifted_lo_aux_1);
    let shifted_hi_is_valid = shifted_hi_0 * (one - shifted_hi_aux_1);

    yield_constr.constraint(filter * shifted_lo_is_valid);
    yield_constr.constraint(filter * shifted_hi_is_valid);

    // Check
    // 2^exp * input_lo == shifted_lo_0 + 2^32 * shifted_lo_1
    // 2^exp * input_hi == shifted_hi_0 + 2^32 * shifted_hi_1

    let base = F::from_canonical_u64(1u64 << 32);
    let shifted_lo = shifted_lo_0 + shifted_lo_1 * base;
    let shifted_hi = shifted_hi_0 + shifted_hi_1 * base;

    // exp must be <= 32 for this to never overflow in
    // GoldilocksField: since 0 <= input_{lo,hi} <= 2^32 - 1,
    // input_{lo,hi} * 2^32 <= 2^64 - 2^32 < 2^64 - 2^32 + 1 = Goldilocks.
    let shifted_lo_expected = input_lo * pow_exp;
    let shifted_hi_expected = input_hi * pow_exp;

    yield_constr.constraint(filter * (shifted_lo_expected - shifted_lo));
    yield_constr.constraint(filter * (shifted_hi_expected - shifted_hi));

    (
        top_bit,
        shifted_lo_0,
        shifted_lo_1,
        shifted_hi_0,
        shifted_hi_1,
        output_lo,
        output_hi,
    )
}

pub(crate) fn eval_rotate_left<F: Field, P: PackedField<Scalar = F>>(
    lv: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_rol = lv[IS_ROTATE_LEFT];
    let one = P::ONES;

    let (delta_ge32, shifted_lo_0, shifted_lo_1, shifted_hi_0, shifted_hi_1, output_lo, output_hi) =
        eval_rotate_shift(lv, yield_constr, is_rol);

    // Intuitively we want to do this (which works when delta <= 32):
    //let lo_constr = shifted_hi_1 + shifted_lo_0 - output_lo;
    //let hi_constr = shifted_lo_1 + shifted_hi_0 - output_hi;

    // If delta_bits[5] == 0, then delta < 32, so we use the bottom term.
    // Otherwise delta_bits[5] == 1, so 32 <= delta < 64 and we need
    // to swap the constraints for the hi and lo halves; hence we use
    // the bottom term which is the top term from hi_constr.
    let lo_constr = (one - delta_ge32) * (shifted_hi_1 + shifted_lo_0 - output_lo)
        + delta_ge32 * (shifted_lo_1 + shifted_hi_0 - output_lo);
    let hi_constr = (one - delta_ge32) * (shifted_lo_1 + shifted_hi_0 - output_hi)
        + delta_ge32 * (shifted_hi_1 + shifted_lo_0 - output_hi);
    yield_constr.constraint(is_rol * lo_constr);
    yield_constr.constraint(is_rol * hi_constr);
}

pub(crate) fn eval_rotate_right<F: Field, P: PackedField<Scalar = F>>(
    lv: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_ror = lv[IS_ROTATE_RIGHT];
    let one = P::ONES;

    let (delta_ge32, shifted_lo_0, shifted_lo_1, shifted_hi_0, shifted_hi_1, output_lo, output_hi) =
        eval_rotate_shift(lv, yield_constr, is_ror);

    // Intuitively we want to do this (which works when delta <= 32):
    // let lo_constr = shifted_lo_1 + shifted_hi_0 - output_lo;
    // let hi_constr = shifted_hi_1 + shifted_lo_0 - output_hi;

    let lo_constr = (one - delta_ge32) * (shifted_lo_1 + shifted_hi_0 - output_lo)
        + delta_ge32 * (shifted_hi_1 + shifted_lo_0 - output_lo);
    let hi_constr = (one - delta_ge32) * (shifted_hi_1 + shifted_lo_0 - output_hi)
        + delta_ge32 * (shifted_lo_1 + shifted_hi_0 - output_hi);
    yield_constr.constraint(is_ror * lo_constr);
    yield_constr.constraint(is_ror * hi_constr);
}

pub(crate) fn eval_shift_left<F: Field, P: PackedField<Scalar = F>>(
    lv: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_shl = lv[IS_SHIFT_LEFT];
    let one = P::ONES;

    let (delta_ge32, shifted_lo_0, shifted_lo_1, shifted_hi_0, _shifted_hi_1, output_lo, output_hi) =
        eval_rotate_shift(lv, yield_constr, is_shl);

    // Intuitively we want to do this (which works when delta <= 32):
    //let lo_constr = shifted_lo_0 - output_lo;
    //let hi_constr = shifted_lo_1 + shifted_hi_0 - output_hi;

    let lo_constr =
        (one - delta_ge32) * (shifted_lo_0 - output_lo) + delta_ge32 * (P::ZEROS - output_lo);
    let hi_constr = (one - delta_ge32) * (shifted_lo_1 + shifted_hi_0 - output_hi)
        + delta_ge32 * (shifted_lo_0 - output_hi);
    yield_constr.constraint(is_shl * lo_constr);
    yield_constr.constraint(is_shl * hi_constr);
}

pub(crate) fn eval_shift_right<F: Field, P: PackedField<Scalar = F>>(
    lv: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_shl = lv[IS_SHIFT_LEFT];
    let one = P::ONES;

    let (delta_ge32, _shifted_lo_0, shifted_lo_1, shifted_hi_0, shifted_hi_1, output_lo, output_hi) =
        eval_rotate_shift(lv, yield_constr, is_shl);

    // Intuitively we want to do this (which works when delta <= 32):
    //let lo_constr = shifted_lo_1 + shifted_hi_0 - output_hi;
    //let hi_constr = shifted_hi_1 - output_lo;

    let lo_constr = (one - delta_ge32) * (shifted_lo_1 + shifted_hi_0 - output_lo)
        + delta_ge32 * (shifted_hi_1 - output_lo);
    let hi_constr =
        (one - delta_ge32) * (shifted_hi_1 - output_hi) + delta_ge32 * (P::ZEROS - output_hi);
    yield_constr.constraint(is_shl * lo_constr);
    yield_constr.constraint(is_shl * hi_constr);
}

pub(crate) fn eval_rotate_shift_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    filter: ExtensionTarget<D>,
) -> (
    ExtensionTarget<D>,
    ExtensionTarget<D>,
    ExtensionTarget<D>,
    ExtensionTarget<D>,
    ExtensionTarget<D>,
    ExtensionTarget<D>,
    ExtensionTarget<D>,
) {
    // Input
    let input_lo = lv[COL_ROTATE_SHIFT_INPUT_LO];
    let input_hi = lv[COL_ROTATE_SHIFT_INPUT_HI];

    let exp_bits = COL_ROTATE_SHIFT_EXP_BITS.map(|r| lv[r]);
    let top_bit = lv[COL_ROTATE_SHIFT_DELTA_DIV32];

    let pow_exp_aux_0 = lv[COL_ROTATE_SHIFT_POW_EXP_AUX_0];
    let pow_exp_aux_1 = lv[COL_ROTATE_SHIFT_POW_EXP_AUX_1];
    let pow_exp_aux_2 = lv[COL_ROTATE_SHIFT_POW_EXP_AUX_2];
    let pow_exp = lv[COL_ROTATE_SHIFT_POW_EXP];

    let shifted_lo_0 = lv[COL_ROTATE_SHIFT_DISPLACED_INPUT_LO_0];
    let shifted_lo_1 = lv[COL_ROTATE_SHIFT_DISPLACED_INPUT_LO_1];
    let shifted_hi_0 = lv[COL_ROTATE_SHIFT_DISPLACED_INPUT_HI_0];
    let shifted_hi_1 = lv[COL_ROTATE_SHIFT_DISPLACED_INPUT_HI_1];
    let shifted_lo_aux_0 = lv[COL_ROTATE_SHIFT_DISPLACED_INPUT_LO_AUX_0];
    let shifted_lo_aux_1 = lv[COL_ROTATE_SHIFT_DISPLACED_INPUT_LO_AUX_1];
    let shifted_hi_aux_0 = lv[COL_ROTATE_SHIFT_DISPLACED_INPUT_HI_AUX_0];
    let shifted_hi_aux_1 = lv[COL_ROTATE_SHIFT_DISPLACED_INPUT_HI_AUX_1];

    // Output
    let output_lo = lv[COL_ROTATE_SHIFT_OUTPUT_0];
    let output_hi = lv[COL_ROTATE_SHIFT_OUTPUT_1];

    // Check that every "bit" of exp_bits and top_bit is 0 or 1
    constrain_all_to_bits(builder, yield_constr, filter, &exp_bits);
    constrain_all_to_bits(builder, yield_constr, filter, &[top_bit]);

    let one = builder.one_extension();
    // c[i-1] = 2^(2^i) - 1
    let c = [1, 2, 3, 4].map(|i| F::from_canonical_u64(1u64 << (1u32 << i)) - F::ONE);

    let constr1 = {
        let t0 = builder.add_extension(exp_bits[0], one);
        let t1 = builder.mul_const_add_extension(c[0], exp_bits[1], one);
        let t2 = builder.mul_sub_extension(t0, t1, pow_exp_aux_0);
        builder.mul_extension(filter, t2)
    };
    yield_constr.constraint(builder, constr1);
    let constr2 = {
        let t0 = builder.mul_const_add_extension(c[1], exp_bits[2], one);
        let t1 = builder.mul_const_add_extension(c[2], exp_bits[3], one);
        let t2 = builder.mul_sub_extension(t0, t1, pow_exp_aux_1);
        builder.mul_extension(filter, t2)
    };
    yield_constr.constraint(builder, constr2);
    let constr3 = {
        let t0 = builder.mul_sub_extension(pow_exp_aux_0, pow_exp_aux_1, pow_exp_aux_2);
        builder.mul_extension(filter, t0)
    };
    yield_constr.constraint(builder, constr3);
    let constr4 = {
        let t0 = builder.mul_const_add_extension(c[3], exp_bits[4], one);
        let t1 = builder.mul_extension(pow_exp_aux_2, t0);
        builder.mul_extension(filter, t1)
    };
    yield_constr.constraint(builder, constr4);

    let u32_max = builder.constant_extension(F::Extension::from_canonical_u32(u32::MAX));

    let constr1 = {
        let t0 = builder.sub_extension(u32_max, shifted_lo_1);
        let t1 = builder.mul_sub_extension(shifted_lo_aux_0, t0, shifted_lo_aux_1);
        builder.mul_extension(filter, t1)
    };
    yield_constr.constraint(builder, constr1);

    let constr2 = {
        let t0 = builder.sub_extension(u32_max, shifted_hi_1);
        let t1 = builder.mul_sub_extension(shifted_hi_aux_0, t0, shifted_hi_aux_1);
        builder.mul_extension(filter, t1)
    };
    yield_constr.constraint(builder, constr2);

    let shifted_lo_is_valid = {
        let t0 = builder.sub_extension(one, shifted_lo_aux_1);
        let t1 = builder.mul_extension(t0, shifted_lo_0);
        builder.mul_extension(filter, t1)
    };
    let shifted_hi_is_valid = {
        let t0 = builder.sub_extension(one, shifted_hi_aux_1);
        let t1 = builder.mul_extension(t0, shifted_hi_0);
        builder.mul_extension(filter, t1)
    };
    yield_constr.constraint(builder, shifted_lo_is_valid);
    yield_constr.constraint(builder, shifted_hi_is_valid);

    let base = builder.constant_extension(F::Extension::from_canonical_u64(1u64 << 32));
    let shifted_lo = builder.mul_add_extension(shifted_lo_1, base, shifted_lo_0);
    let shifted_hi = builder.mul_add_extension(shifted_hi_1, base, shifted_hi_0);

    let shifted_lo_expected = builder.mul_extension(input_lo, pow_exp);
    let shifted_hi_expected = builder.mul_extension(input_hi, pow_exp);

    let shifted_lo_valid = {
        let t0 = builder.sub_extension(shifted_lo_expected, shifted_lo);
        builder.mul_extension(filter, t0)
    };
    yield_constr.constraint(builder, shifted_lo_valid);
    let shifted_hi_valid = {
        let t0 = builder.sub_extension(shifted_hi_expected, shifted_hi);
        builder.mul_extension(filter, t0)
    };
    yield_constr.constraint(builder, shifted_hi_valid);

    (
        top_bit,
        shifted_lo_0,
        shifted_lo_1,
        shifted_hi_0,
        shifted_hi_1,
        output_lo,
        output_hi,
    )
}

pub(crate) fn eval_rotate_left_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_rol = lv[IS_ROTATE_LEFT];

    let (delta_ge32, shifted_lo_0, shifted_lo_1, shifted_hi_0, shifted_hi_1, output_lo, output_hi) =
        eval_rotate_shift_recursively(builder, lv, yield_constr, is_rol);

    let one = builder.one_extension();
    let s0 = builder.add_extension(shifted_hi_1, shifted_lo_0);
    let s1 = builder.add_extension(shifted_lo_1, shifted_hi_0);
    let c = builder.sub_extension(one, delta_ge32);

    let lo_constr = {
        let t0 = builder.sub_extension(s0, output_lo);
        let t1 = builder.mul_extension(c, t0);
        let t2 = builder.sub_extension(s1, output_lo);
        let t3 = builder.mul_extension(delta_ge32, t2);
        let t4 = builder.add_extension(t1, t3);
        builder.mul_extension(is_rol, t4)
    };

    let hi_constr = {
        let t0 = builder.sub_extension(s1, output_hi);
        let t1 = builder.mul_extension(c, t0);
        let t2 = builder.sub_extension(s0, output_hi);
        let t3 = builder.mul_extension(delta_ge32, t2);
        let t4 = builder.add_extension(t1, t3);
        builder.mul_extension(is_rol, t4)
    };

    yield_constr.constraint(builder, lo_constr);
    yield_constr.constraint(builder, hi_constr);
}

pub(crate) fn eval_rotate_right_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_ror = lv[IS_ROTATE_RIGHT];

    let (delta_ge32, shifted_lo_0, shifted_lo_1, shifted_hi_0, shifted_hi_1, output_lo, output_hi) =
        eval_rotate_shift_recursively(builder, lv, yield_constr, is_ror);

    let one = builder.one_extension();
    let s0 = builder.add_extension(shifted_hi_1, shifted_lo_0);
    let s1 = builder.add_extension(shifted_lo_1, shifted_hi_0);
    let c = builder.sub_extension(one, delta_ge32);

    let lo_constr = {
        let t0 = builder.sub_extension(s1, output_lo);
        let t1 = builder.mul_extension(c, t0);
        let t2 = builder.sub_extension(s0, output_lo);
        let t3 = builder.mul_extension(delta_ge32, t2);
        let t4 = builder.add_extension(t1, t3);
        builder.mul_extension(is_ror, t4)
    };

    let hi_constr = {
        let t0 = builder.sub_extension(s0, output_hi);
        let t1 = builder.mul_extension(c, t0);
        let t2 = builder.sub_extension(s1, output_hi);
        let t3 = builder.mul_extension(delta_ge32, t2);
        let t4 = builder.add_extension(t1, t3);
        builder.mul_extension(is_ror, t4)
    };

    yield_constr.constraint(builder, lo_constr);
    yield_constr.constraint(builder, hi_constr);
}

pub(crate) fn eval_shift_left_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_shl = lv[IS_SHIFT_LEFT];

    let (delta_ge32, shifted_lo_0, shifted_lo_1, shifted_hi_0, _shifted_hi_1, output_lo, output_hi) =
        eval_rotate_shift_recursively(builder, lv, yield_constr, is_shl);

    let one = builder.one_extension();
    let c = builder.sub_extension(one, delta_ge32);

    let lo_constr = {
        let t0 = builder.sub_extension(shifted_lo_0, output_lo);
        let t1 = builder.mul_extension(c, t0);
        let t2 = builder.mul_extension(delta_ge32, output_lo);
        let t3 = builder.add_extension(t1, t2);
        builder.mul_extension(is_shl, t3)
    };

    let hi_constr = {
        let t0 = builder.add_extension(shifted_lo_1, shifted_hi_0);
        let t1 = builder.sub_extension(t0, output_hi);
        let t2 = builder.mul_extension(c, t1);
        let t3 = builder.sub_extension(shifted_lo_0, output_hi);
        let t4 = builder.mul_extension(delta_ge32, t3);
        let t5 = builder.add_extension(t2, t4);
        builder.mul_extension(is_shl, t5)
    };

    yield_constr.constraint(builder, lo_constr);
    yield_constr.constraint(builder, hi_constr);
}

pub(crate) fn eval_shift_right_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_shr = lv[IS_SHIFT_RIGHT];

    let (delta_ge32, _shifted_lo_0, shifted_lo_1, shifted_hi_0, shifted_hi_1, output_lo, output_hi) =
        eval_rotate_shift_recursively(builder, lv, yield_constr, is_shr);

    let one = builder.one_extension();
    let c = builder.sub_extension(one, delta_ge32);

    let lo_constr = {
        let t0 = builder.add_extension(shifted_lo_1, shifted_hi_0);
        let t1 = builder.sub_extension(t0, output_lo);
        let t2 = builder.mul_extension(c, t1);
        let t3 = builder.sub_extension(shifted_hi_1, output_lo);
        let t4 = builder.mul_extension(delta_ge32, t3);
        let t5 = builder.add_extension(t2, t4);
        builder.mul_extension(is_shr, t5)
    };

    let hi_constr = {
        let t0 = builder.sub_extension(shifted_hi_1, output_hi);
        let t1 = builder.mul_extension(c, t0);
        let t2 = builder.mul_extension(delta_ge32, output_hi);
        let t3 = builder.add_extension(t1, t2);
        builder.mul_extension(is_shr, t3)
    };

    yield_constr.constraint(builder, lo_constr);
    yield_constr.constraint(builder, hi_constr);
}
