use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::{Field, PrimeField64};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

use crate::alu::bitops::{binary_to_u32, constrain_all_to_bits};
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

pub(crate) fn generate_rotate_shift<F: PrimeField64>(
    values: &mut [F; NUM_COLUMNS],
    op: usize,
) {
    // input_{lo,hi} are the 32-bit lo and hi words of the input
    let input_lo = values[COL_ROTATE_SHIFT_INPUT_LO].to_canonical_u64();
    let input_hi = values[COL_ROTATE_SHIFT_INPUT_HI].to_canonical_u64();

    // delta: displacement amount
    let delta_bits = COL_ROTATE_SHIFT_DELTA_BITS.map(|r| values[r]);
    let delta = binary_to_u32(delta_bits).to_canonical_u64();
    let delta_mod32 = delta % 32;
    let delta_bits = delta_bits.map(|b| b.to_canonical_u64());

    // helper values
    let pow_delta_aux_0 = (delta_bits[0] + 1) * (3 * delta_bits[1] + 1);
    let pow_delta_aux_1 = (15 * delta_bits[2] + 1) * (255 * delta_bits[3] + 1);
    let pow_delta_aux_2 = pow_delta_aux_0 * pow_delta_aux_1;
    let pow_delta_mod32 = pow_delta_aux_2 * (65535 * delta_bits[4] + 1);
    values[COL_ROTATE_SHIFT_POW_DELTA_AUX_0] = F::from_canonical_u64(pow_delta_aux_0);
    values[COL_ROTATE_SHIFT_POW_DELTA_AUX_1] = F::from_canonical_u64(pow_delta_aux_1);
    values[COL_ROTATE_SHIFT_POW_DELTA_AUX_2] = F::from_canonical_u64(pow_delta_aux_2);
    values[COL_ROTATE_SHIFT_POW_DELTA_MOD32] = F::from_canonical_u64(pow_delta_mod32);

    let shifted_lo = input_lo << delta_mod32;
    let shifted_lo_0 = shifted_lo as u32;
    let shifted_lo_1 = (shifted_lo >> 32) as u32;
    values[COL_ROTATE_SHIFT_DISPLACED_INPUT_LO_0] = F::from_canonical_u32(shifted_lo_0);
    values[COL_ROTATE_SHIFT_DISPLACED_INPUT_LO_1] = F::from_canonical_u32(shifted_lo_1);
    let shifted_hi = input_hi << delta_mod32;
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
) -> ([P; 6], P, P, P, P, P, P) {
    // Input
    let input_lo = lv[COL_ROTATE_SHIFT_INPUT_LO];
    let input_hi = lv[COL_ROTATE_SHIFT_INPUT_HI];

    // Delta is the shift/rotate displacement
    let delta_bits = COL_ROTATE_SHIFT_DELTA_BITS.map(|r| lv[r]);
    let pow_delta_aux_0 = lv[COL_ROTATE_SHIFT_POW_DELTA_AUX_0];
    let pow_delta_aux_1 = lv[COL_ROTATE_SHIFT_POW_DELTA_AUX_1];
    let pow_delta_aux_2 = lv[COL_ROTATE_SHIFT_POW_DELTA_AUX_2];
    let pow_delta_mod32 = lv[COL_ROTATE_SHIFT_POW_DELTA_MOD32];

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

    // Check that every "bit" of delta_bits is 0 or 1
    delta_bits.map(|b| yield_constr.constraint(filter * (b * b - b)));

    // Check that pow_delta_mod32 = 2^delta_mod32, where delta_mod32
    // is formed from the bits delta_bits[0..4].
    //
    // 2^delta_mod32 = \prod_i=0^4 (2^(2^i) if delta_bits[i] = 1 else 1)
    //               = \prod_i=0^4 ((2^(2^i) - 1) * delta_bits[i] + 1)
    //               = pow_delta_mod32
    //
    // on the conditions that:
    //
    // pow_delta_aux_0 = \prod_i=0^1 ((2^i - 1) * delta_bits[i] + 1)
    // pow_delta_aux_1 = \prod_i=2^3 ((2^i - 1) * delta_bits[i] + 1)
    // pow_delta_aux_2 = pow_delta_aux_0 * pow_delta_aux_1
    // pow_delta_mod32 = pow_delta_aux_2 * ((2^(2^4) - 1) * delta_bits[4] + 1)

    let one = P::ONES;
    // c[i-1] = 2^(2^i) - 1
    let c = [1, 2, 3, 4]
        .map(|i| P::from(F::from_canonical_u64(1u64 << (1u32 << i))) - P::ONES);

    let constr1 = (delta_bits[0] + one) * (c[0] * delta_bits[1] + one);
    yield_constr.constraint(filter * (constr1 - pow_delta_aux_0));
    let constr2 = (c[1] * delta_bits[2] + one) * (c[2] * delta_bits[3] + one);
    yield_constr.constraint(filter * (constr2 - pow_delta_aux_1));
    let constr3 = pow_delta_aux_0 * pow_delta_aux_1;
    yield_constr.constraint(filter * (constr3 - pow_delta_aux_2));
    let constr4 = pow_delta_aux_2 * (c[3] * delta_bits[4] + one);
    yield_constr.constraint(filter * (constr4 - pow_delta_mod32));

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
    // 2^delta_mod32 * input_lo == shifted_lo_0 + 2^32 * shifted_lo_1
    // 2^delta_mod32 * input_hi == shifted_hi_0 + 2^32 * shifted_hi_1

    let base = F::from_canonical_u64(1u64 << 32);
    let shifted_lo = shifted_lo_0 + shifted_lo_1 * base;
    let shifted_hi = shifted_hi_0 + shifted_hi_1 * base;

    // delta must be <= 32 for this to never overflow in
    // GoldilocksField: since 0 <= input_{lo,hi} <= 2^32 - 1,
    // input_{lo,hi} * 2^32 <= 2^64 - 2^32 < 2^64 - 2^32 + 1 = Goldilocks.
    let shifted_lo_expected = input_lo * pow_delta_mod32;
    let shifted_hi_expected = input_hi * pow_delta_mod32;

    yield_constr.constraint(filter * (shifted_lo_expected - shifted_lo));
    yield_constr.constraint(filter * (shifted_hi_expected - shifted_hi));

    (delta_bits, shifted_lo_0, shifted_lo_1, shifted_hi_0, shifted_hi_1, output_lo, output_hi)
    /*
    Other operation constraints for when delta < 32:

    /// ROTATE RIGHT
    // Do rotate/shift to the right by delta using rotate/shift to the
    // left by 32 - delta.

    // ?? same as for rotate left but the lo constr is the hi and v.v.
    let lo_constr = shifted_lo_1 + shifted_hi_0 - output_hi;
    let hi_constr = shifted_hi_1 + shifted_lo_0 - output_lo;
    yield_constr.constraint(is_rotl * lo_constr);
    yield_constr.constraint(is_rotl * hi_constr);

    /// SHIFT RIGHT
    let lo_constr = shifted_lo_1 + shifted_hi_0 - output_hi;
    let hi_constr = shifted_hi_1 - output_lo;
    yield_constr.constraint(is_rotl * lo_constr);
    yield_constr.constraint(is_rotl * hi_constr);
    */
}

pub(crate) fn eval_rotate_left<F: Field, P: PackedField<Scalar = F>>(
    lv: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_rol = lv[IS_ROTATE_LEFT];
    let one = P::ONES;

    let (delta_bits, shifted_lo_0, shifted_lo_1, shifted_hi_0, shifted_hi_1, output_lo, output_hi) = eval_rotate_shift(lv, yield_constr, is_rol);

    // Intuitively we want to do this (which works when delta_mod32 <= 32):
    //let lo_constr = shifted_hi_1 + shifted_lo_0 - output_lo;
    //let hi_constr = shifted_lo_1 + shifted_hi_0 - output_hi;

    // If delta_bits[5] == 0, then delta < 32, so we use the bottom term.
    // Otherwise delta_bits[5] == 1, so 32 <= delta < 64 and we need
    // to swap the constraints for the hi and lo halves; hence we use
    // the bottom term which is the top term from hi_constr.
    let lo_constr =
        (one - delta_bits[5]) * (shifted_hi_1 + shifted_lo_0 - output_lo)
        + delta_bits[5] * (shifted_lo_1 + shifted_hi_0 - output_lo);
    let hi_constr =
        (one - delta_bits[5]) * (shifted_lo_1 + shifted_hi_0 - output_hi)
        + delta_bits[5] * (shifted_hi_1 + shifted_lo_0 - output_hi);
    yield_constr.constraint(is_rol * lo_constr);
    yield_constr.constraint(is_rol * hi_constr);
}

pub(crate) fn eval_shift_left<F: Field, P: PackedField<Scalar = F>>(
    lv: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_shl = lv[IS_SHIFT_LEFT];
    let one = P::ONES;

    let (delta_bits, shifted_lo_0, shifted_lo_1, shifted_hi_0, _shifted_hi_1, output_lo, output_hi) = eval_rotate_shift(lv, yield_constr, is_shl);

    // Intuitively we want to do this (which works when delta_mod32 <= 32):
    //let lo_constr = shifted_lo_0 - output_lo;
    //let hi_constr = shifted_lo_1 + shifted_hi_0 - output_hi;

    let lo_constr =
        (one - delta_bits[5]) * (shifted_lo_0 - output_lo)
        + delta_bits[5] * output_lo;
    let hi_constr =
        (one - delta_bits[5]) * (shifted_lo_1 + shifted_hi_0 - output_hi)
        + delta_bits[5] * (shifted_lo_1 + shifted_lo_0 - output_hi);
    yield_constr.constraint(is_shl * lo_constr);
    yield_constr.constraint(is_shl * hi_constr);
}

pub(crate) fn eval_rotate_shift_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    filter: ExtensionTarget<D>,
) -> ([ExtensionTarget<D>; 6], ExtensionTarget<D>, ExtensionTarget<D>, ExtensionTarget<D>, ExtensionTarget<D>, ExtensionTarget<D>, ExtensionTarget<D>) {
    // Input
    let input_lo = lv[COL_ROTATE_SHIFT_INPUT_LO];
    let input_hi = lv[COL_ROTATE_SHIFT_INPUT_HI];

    let delta_bits = COL_ROTATE_SHIFT_DELTA_BITS.map(|r| lv[r]);
    let pow_delta_aux_0 = lv[COL_ROTATE_SHIFT_POW_DELTA_AUX_0];
    let pow_delta_aux_1 = lv[COL_ROTATE_SHIFT_POW_DELTA_AUX_1];
    let pow_delta_aux_2 = lv[COL_ROTATE_SHIFT_POW_DELTA_AUX_2];
    let pow_delta_mod32 = lv[COL_ROTATE_SHIFT_POW_DELTA_MOD32];

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

    // Check that every "bit" of delta_bits is 0 or 1
    constrain_all_to_bits(builder, yield_constr, filter, &delta_bits);

    // c[i-1] = 2^(2^i) - 1
    let c = [1, 2, 3, 4]
        .map(|i| F::from_canonical_u64(1u64 << (1u32 << i)) - F::ONE);
    let one = builder.one_extension();

    let constr1 = {
        let t0 = builder.add_extension(delta_bits[0], one);
        let t1 = builder.mul_const_add_extension(c[0], delta_bits[1], one);
        let t2 = builder.mul_sub_extension(t0, t1, pow_delta_aux_0);
        builder.mul_extension(filter, t2)
    };
    yield_constr.constraint(builder, constr1);
    let constr2 = {
        let t0 = builder.mul_const_add_extension(c[1], delta_bits[2], one);
        let t1 = builder.mul_const_add_extension(c[2], delta_bits[3], one);
        let t2 = builder.mul_sub_extension(t0, t1, pow_delta_aux_1);
        builder.mul_extension(filter, t2)
    };
    yield_constr.constraint(builder, constr2);
    let constr3 = {
        let t0 = builder.mul_sub_extension(pow_delta_aux_0, pow_delta_aux_1, pow_delta_aux_2);
        builder.mul_extension(filter, t0)
    };
    yield_constr.constraint(builder, constr3);
    let constr4 = {
        let t0 = builder.mul_const_add_extension(c[3], delta_bits[4], one);
        let t1 = builder.mul_extension(pow_delta_aux_2, t0);
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

    let shifted_lo_expected = builder.mul_extension(input_lo, pow_delta_mod32);
    let shifted_hi_expected = builder.mul_extension(input_hi, pow_delta_mod32);

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

    (delta_bits, shifted_lo_0, shifted_lo_1, shifted_hi_0, shifted_hi_1, output_lo, output_hi)
}

pub(crate) fn eval_rotate_left_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_rotl = lv[IS_ROTATE_LEFT];

    let (delta_bits, shifted_lo_0, shifted_lo_1, shifted_hi_0, shifted_hi_1, output_lo, output_hi) = eval_rotate_shift_recursively(builder, lv, yield_constr, is_rotl);

    let one = builder.one_extension();
    let s0 = builder.add_extension(shifted_hi_1, shifted_lo_0);
    let s1 = builder.add_extension(shifted_lo_1, shifted_hi_0);
    let c = builder.sub_extension(one, delta_bits[5]);

    let lo_constr = {
        let t0 = builder.sub_extension(s0, output_lo);
        let t1 = builder.mul_extension(c, t0);
        let t2 = builder.sub_extension(s1, output_lo);
        let t3 = builder.mul_extension(delta_bits[5], t2);
        let t4 = builder.add_extension(t1, t3);
        builder.mul_extension(is_rotl, t4)
    };

    let hi_constr = {
        let t0 = builder.sub_extension(s1, output_hi);
        let t1 = builder.mul_extension(c, t0);
        let t2 = builder.sub_extension(s0, output_hi);
        let t3 = builder.mul_extension(delta_bits[5], t2);
        let t4 = builder.add_extension(t1, t3);
        builder.mul_extension(is_rotl, t4)
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

    let (delta_bits, shifted_lo_0, shifted_lo_1, shifted_hi_0, _shifted_hi_1, output_lo, output_hi) = eval_rotate_shift_recursively(builder, lv, yield_constr, is_shl);

    let one = builder.one_extension();
    let c = builder.sub_extension(one, delta_bits[5]);

    let lo_constr = {
        let t0 = builder.sub_extension(shifted_lo_0, output_lo);
        let t1 = builder.mul_extension(c, t0);
        let t2 = builder.mul_extension(delta_bits[5], output_lo);
        let t3 = builder.add_extension(t1, t2);
        builder.mul_extension(is_shl, t3)
    };

    let hi_constr = {
        let t0 = builder.add_extension(shifted_lo_1, shifted_hi_0);
        let t1 = builder.sub_extension(t0, output_hi);
        let t2 = builder.mul_extension(c, t1);
        let t3 = builder.add_extension(shifted_lo_0, shifted_lo_1);
        let t4 = builder.sub_extension(t3, output_hi);
        let t5 = builder.mul_extension(delta_bits[5], t4);
        let t6 = builder.add_extension(t2, t5);
        builder.mul_extension(is_shl, t6)
    };

    yield_constr.constraint(builder, lo_constr);
    yield_constr.constraint(builder, hi_constr);
}
