use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::{Field, PrimeField64};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};

use crate::alu::bitops::constrain_all_to_bits_circuit;
use crate::registers::alu::*;
use crate::registers::NUM_COLUMNS;

/// ROTATE and SHIFT instructions
///
/// To rotate a 64bit value by DELTA bit positions, the input is
///
/// - a 64-bit integer X to be rotated/shifted, given as high and low 32-bit
///   words X_lo and X_hi.
/// - a 32-bit integer EXP (given as its 5 bits) which is either DELTA
///   mod 32, if the operation direction is left, or (32 - (DELTA mod 32))
///   mod 32 if the operation direction is right.
/// - a single bit DELTA_DIV32 which is 1 if DELTA is >= 32 and 0 otherwise
/// - the value POW_EXP = 2^EXP, as well as three auxiliary values POW_EXP_AUX_[012]
///   to verify that POW_EXP == 2^EXP
/// - two 64-bit integers, INPUT_LO_DISPLACED and INPUT_HI_DISPLACED,
///   with INPUT_LO_DISPLACED being the high and low 32-bit words of
///   the value 2^EXP * X_lo; similarly for INPUT_HI_DISPLACED.
/// - two 64-bit auxiliary values DISPLACED_INPUT_{LO,HI}_AUX, one
///   each for INPUT_LO_DISPLACED and INPUT_HI_DISPLACED, used to prove
///   that INPUT_LO_DISPLACED and INPUT_HI_DISPLACED are valid
///   Goldilocks elements.

pub(crate) fn generate_rotate_shift<F: PrimeField64>(values: &mut [F; NUM_COLUMNS], op: usize) {
    // input_{lo,hi} are the 32-bit lo and hi words of the input
    let input_lo = values[COL_ROTATE_SHIFT_INPUT_LO].to_canonical_u64();
    let input_hi = values[COL_ROTATE_SHIFT_INPUT_HI].to_canonical_u64();

    // Given the 6 bits delta_bits[0..5], bits 0..4 represent
    // delta_mod32 for left ops and (32 - delta_mod32) % 32 for right
    // ops, and delta_bits[5] represents whether delta >= 32.

    // delta is the displacement amount. EXP_BITS holds the 5 bits of
    // either delta mod 32 (for left ops) or (32 - (delta mod 32)) mod 32
    // for right ops.
    let exp_bits = COL_ROTATE_SHIFT_EXP_BITS.map(|r| values[r].to_canonical_u64());

    let is_right_op = op == IS_ROTATE_RIGHT || op == IS_SHIFT_RIGHT || op == IS_ARITH_SHIFT_RIGHT;
    let exp: u64 = [0, 1, 2, 3, 4].map(|i| exp_bits[i] << i).into_iter().sum();
    let delta_mod32 = if is_right_op { (32u64 - exp) % 32 } else { exp };
    let exp_ge32_bit = values[COL_ROTATE_SHIFT_DELTA_DIV32].to_canonical_u64();
    let delta = (exp_ge32_bit << 5) + delta_mod32;

    // helper values
    let pow_exp_aux_0 = (exp_bits[0] + 1) * (3 * exp_bits[1] + 1);
    let pow_exp_aux_1 = (15 * exp_bits[2] + 1) * (255 * exp_bits[3] + 1);
    let pow_exp_aux_2 = pow_exp_aux_0 * pow_exp_aux_1;
    let pow_exp = pow_exp_aux_2 * (65535 * exp_bits[4] + 1);
    values[COL_ROTATE_SHIFT_POW_EXP_AUX_0] = F::from_canonical_u64(pow_exp_aux_0);
    values[COL_ROTATE_SHIFT_POW_EXP_AUX_1] = F::from_canonical_u64(pow_exp_aux_1);
    values[COL_ROTATE_SHIFT_POW_EXP_AUX_2] = F::from_canonical_u64(pow_exp_aux_2);
    values[COL_ROTATE_SHIFT_POW_EXP] = F::from_canonical_u64(pow_exp);

    let lo_shifted = input_lo << exp;
    let lo_shifted_0 = lo_shifted as u32;
    let lo_shifted_1 = (lo_shifted >> 32) as u32;
    values[COL_ROTATE_SHIFT_INPUT_LO_DISPLACED_0] = F::from_canonical_u32(lo_shifted_0);
    values[COL_ROTATE_SHIFT_INPUT_LO_DISPLACED_1] = F::from_canonical_u32(lo_shifted_1);
    let hi_shifted = input_hi << exp;
    let hi_shifted_0 = hi_shifted as u32;
    let hi_shifted_1 = (hi_shifted >> 32) as u32;
    values[COL_ROTATE_SHIFT_INPUT_HI_DISPLACED_0] = F::from_canonical_u32(hi_shifted_0);
    values[COL_ROTATE_SHIFT_INPUT_HI_DISPLACED_1] = F::from_canonical_u32(hi_shifted_1);

    if lo_shifted_1 != u32::MAX {
        let diff = F::from_canonical_u32(u32::MAX - lo_shifted_1);
        let inv = diff.inverse();
        values[COL_ROTATE_SHIFT_INPUT_LO_DISPLACED_AUX_0] = inv;
        values[COL_ROTATE_SHIFT_INPUT_LO_DISPLACED_AUX_1] = diff * inv;
    } else {
        // lo_shifted_0 must be zero, so this is unused.
        values[COL_ROTATE_SHIFT_INPUT_LO_DISPLACED_AUX_0] = F::ZERO;
        values[COL_ROTATE_SHIFT_INPUT_LO_DISPLACED_AUX_1] = F::ZERO;
    }
    if hi_shifted_1 != u32::MAX {
        let diff = F::from_canonical_u32(u32::MAX - hi_shifted_1);
        let inv = diff.inverse();
        values[COL_ROTATE_SHIFT_INPUT_HI_DISPLACED_AUX_0] = inv;
        values[COL_ROTATE_SHIFT_INPUT_HI_DISPLACED_AUX_1] = diff * inv;
    } else {
        // hi_shifted_0 must be zero, so this is unused.
        values[COL_ROTATE_SHIFT_INPUT_HI_DISPLACED_AUX_0] = F::ZERO;
        values[COL_ROTATE_SHIFT_INPUT_HI_DISPLACED_AUX_1] = F::ZERO;
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

/// Check that pow_exp = 2^exp, where exp is formed from the bits
/// exp_bits[0..4].
///
/// 2^exp = \prod_i=0^4 (2^(2^i) if exp_bits[i] = 1 else 1)
///       = \prod_i=0^4 ((2^(2^i) - 1) * exp_bits[i] + 1)
///       = pow_exp
///
/// on the conditions that:
///
///    pow_exp_aux_0 = \prod_i=0^1 ((2^i - 1) * exp_bits[i] + 1)
///    pow_exp_aux_1 = \prod_i=2^3 ((2^i - 1) * exp_bits[i] + 1)
///    pow_exp_aux_2 = pow_exp_aux_0 * pow_exp_aux_1
///    pow_exp_mod32 = pow_exp_aux_2 * ((2^(2^4) - 1) * exp_bits[4] + 1)
///
/// Also check that every "bit" of exp_bits and exp_ge32_bit is 0 or 1.
fn constrain_pow_exp<F: Field, P: PackedField<Scalar = F>>(
    lv: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
    filter: P,
) {
    let exp_bits = COL_ROTATE_SHIFT_EXP_BITS.map(|r| lv[r]);
    let exp_ge32_bit = lv[COL_ROTATE_SHIFT_DELTA_DIV32];

    let pow_exp_aux_0 = lv[COL_ROTATE_SHIFT_POW_EXP_AUX_0];
    let pow_exp_aux_1 = lv[COL_ROTATE_SHIFT_POW_EXP_AUX_1];
    let pow_exp_aux_2 = lv[COL_ROTATE_SHIFT_POW_EXP_AUX_2];
    let pow_exp = lv[COL_ROTATE_SHIFT_POW_EXP];

    // Check that every "bit" of exp_bits and exp_ge32_bit is 0 or 1
    exp_bits.map(|b| yield_constr.constraint(filter * (b * b - b)));
    yield_constr.constraint(filter * (exp_ge32_bit * exp_ge32_bit - exp_ge32_bit));

    // c[i-1] = 2^(2^i) - 1
    let c = [1, 2, 3, 4].map(|i| P::from(F::from_canonical_u64(1u64 << (1u32 << i))) - P::ONES);

    let constr1 = (exp_bits[0] + P::ONES) * (c[0] * exp_bits[1] + P::ONES);
    yield_constr.constraint(filter * (constr1 - pow_exp_aux_0));
    let constr2 = (c[1] * exp_bits[2] + P::ONES) * (c[2] * exp_bits[3] + P::ONES);
    yield_constr.constraint(filter * (constr2 - pow_exp_aux_1));
    let constr3 = pow_exp_aux_0 * pow_exp_aux_1;
    yield_constr.constraint(filter * (constr3 - pow_exp_aux_2));
    let constr4 = pow_exp_aux_2 * (c[3] * exp_bits[4] + P::ONES);
    yield_constr.constraint(filter * (constr4 - pow_exp));
}

/// An invalid lo_shifted (or _hi) can be too big to fit in Goldilocks
/// field; e.g. if both _0 and _1 parts are 2^32-1, then lo_shifted =
/// 2^32 - 1 + 2^32 (2^32 - 1) = 2^64 - 1 which overflows in
/// GoldilocksField. Hence we check that {lo,hi}_shifted are valid
/// Goldilocks elements following
/// https:///hackmd.io/NC-yRmmtRQSvToTHb96e8Q#Checking-element-validity
///
/// The idea is check that a value v = (v_lo, v_hi) (32-bit words)
/// satisfies the condition (v_lo == 0 OR v_hi != 2^32-1), which uses
/// the structure of Goldilocks to check that v has the right form.
/// The formula is:
///   v_lo * (one - aux * (u32_max - v_hi)) == 0
/// where aux = (m32_max - v_hi)^-1 if it exists.
fn constrain_shifted_are_valid<F: Field, P: PackedField<Scalar = F>>(
    lv: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
    filter: P,
) {
    let lo_shifted_0 = lv[COL_ROTATE_SHIFT_INPUT_LO_DISPLACED_0];
    let lo_shifted_1 = lv[COL_ROTATE_SHIFT_INPUT_LO_DISPLACED_1];
    let hi_shifted_0 = lv[COL_ROTATE_SHIFT_INPUT_HI_DISPLACED_0];
    let hi_shifted_1 = lv[COL_ROTATE_SHIFT_INPUT_HI_DISPLACED_1];
    let lo_shifted_aux_0 = lv[COL_ROTATE_SHIFT_INPUT_LO_DISPLACED_AUX_0];
    let lo_shifted_aux_1 = lv[COL_ROTATE_SHIFT_INPUT_LO_DISPLACED_AUX_1];
    let hi_shifted_aux_0 = lv[COL_ROTATE_SHIFT_INPUT_HI_DISPLACED_AUX_0];
    let hi_shifted_aux_1 = lv[COL_ROTATE_SHIFT_INPUT_HI_DISPLACED_AUX_1];

    // u32_max = 2^32 - 1
    let u32_max = P::from(F::from_canonical_u32(u32::MAX));

    let constr1 = lo_shifted_aux_0 * (u32_max - lo_shifted_1);
    yield_constr.constraint(filter * (constr1 - lo_shifted_aux_1));
    let constr2 = hi_shifted_aux_0 * (u32_max - hi_shifted_1);
    yield_constr.constraint(filter * (constr2 - hi_shifted_aux_1));

    let lo_shifted_is_valid = lo_shifted_0 * (P::ONES - lo_shifted_aux_1);
    let hi_shifted_is_valid = hi_shifted_0 * (P::ONES - hi_shifted_aux_1);

    yield_constr.constraint(filter * lo_shifted_is_valid);
    yield_constr.constraint(filter * hi_shifted_is_valid);
}

fn eval_rotate_shift<F: Field, P: PackedField<Scalar = F>>(
    lv: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
    filter: P,
) -> (P, P, P, P, P, P, P) {
    // Input
    let input_lo = lv[COL_ROTATE_SHIFT_INPUT_LO];
    let input_hi = lv[COL_ROTATE_SHIFT_INPUT_HI];

    // Delta is the shift/rotate displacement; exp is delta mod 32 or
    // (32 - (delta mod 32)) mod 32, depending on whether the operation
    // direction is left or right.
    let exp_ge32_bit = lv[COL_ROTATE_SHIFT_DELTA_DIV32];
    let pow_exp = lv[COL_ROTATE_SHIFT_POW_EXP];

    let lo_shifted_0 = lv[COL_ROTATE_SHIFT_INPUT_LO_DISPLACED_0];
    let lo_shifted_1 = lv[COL_ROTATE_SHIFT_INPUT_LO_DISPLACED_1];
    let hi_shifted_0 = lv[COL_ROTATE_SHIFT_INPUT_HI_DISPLACED_0];
    let hi_shifted_1 = lv[COL_ROTATE_SHIFT_INPUT_HI_DISPLACED_1];

    // Output
    let output_lo = lv[COL_ROTATE_SHIFT_OUTPUT_0];
    let output_hi = lv[COL_ROTATE_SHIFT_OUTPUT_1];

    constrain_pow_exp(lv, yield_constr, filter);
    constrain_shifted_are_valid(lv, yield_constr, filter);

    // Check
    // 2^exp * input_lo == lo_shifted_0 + 2^32 * lo_shifted_1
    // 2^exp * input_hi == hi_shifted_0 + 2^32 * hi_shifted_1

    let base = F::from_canonical_u64(1u64 << 32);
    let lo_shifted = lo_shifted_0 + lo_shifted_1 * base;
    let hi_shifted = hi_shifted_0 + hi_shifted_1 * base;

    // exp must be <= 32 for this to never overflow in
    // GoldilocksField: since 0 <= input_{lo,hi} <= 2^32 - 1,
    // input_{lo,hi} * 2^32 <= 2^64 - 2^32 < 2^64 - 2^32 + 1 = Goldilocks.
    let lo_shifted_expected = input_lo * pow_exp;
    let hi_shifted_expected = input_hi * pow_exp;

    yield_constr.constraint(filter * (lo_shifted_expected - lo_shifted));
    yield_constr.constraint(filter * (hi_shifted_expected - hi_shifted));

    (
        exp_ge32_bit,
        lo_shifted_0,
        lo_shifted_1,
        hi_shifted_0,
        hi_shifted_1,
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

    let (delta_ge32, lo_shifted_0, lo_shifted_1, hi_shifted_0, hi_shifted_1, output_lo, output_hi) =
        eval_rotate_shift(lv, yield_constr, is_rol);

    // Intuitively we want to do this (which works when delta <= 32):
    //let lo_constr = hi_shifted_1 + lo_shifted_0 - output_lo;
    //let hi_constr = lo_shifted_1 + hi_shifted_0 - output_hi;

    // If delta_bits[5] == 0, then delta < 32, so we use the bottom term.
    // Otherwise delta_bits[5] == 1, so 32 <= delta < 64 and we need
    // to swap the constraints for the hi and lo halves; hence we use
    // the bottom term which is the top term from hi_constr.
    let lo_constr = (one - delta_ge32) * (hi_shifted_1 + lo_shifted_0 - output_lo)
        + delta_ge32 * (lo_shifted_1 + hi_shifted_0 - output_lo);
    let hi_constr = (one - delta_ge32) * (lo_shifted_1 + hi_shifted_0 - output_hi)
        + delta_ge32 * (hi_shifted_1 + lo_shifted_0 - output_hi);
    yield_constr.constraint(is_rol * lo_constr);
    yield_constr.constraint(is_rol * hi_constr);
}

pub(crate) fn eval_rotate_right<F: Field, P: PackedField<Scalar = F>>(
    lv: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_ror = lv[IS_ROTATE_RIGHT];
    let one = P::ONES;

    let (delta_ge32, lo_shifted_0, lo_shifted_1, hi_shifted_0, hi_shifted_1, output_lo, output_hi) =
        eval_rotate_shift(lv, yield_constr, is_ror);

    // Intuitively we want to do this (which works when delta <= 32):
    // let lo_constr = lo_shifted_1 + hi_shifted_0 - output_lo;
    // let hi_constr = hi_shifted_1 + lo_shifted_0 - output_hi;

    let lo_constr = (one - delta_ge32) * (lo_shifted_1 + hi_shifted_0 - output_lo)
        + delta_ge32 * (hi_shifted_1 + lo_shifted_0 - output_lo);
    let hi_constr = (one - delta_ge32) * (hi_shifted_1 + lo_shifted_0 - output_hi)
        + delta_ge32 * (lo_shifted_1 + hi_shifted_0 - output_hi);
    yield_constr.constraint(is_ror * lo_constr);
    yield_constr.constraint(is_ror * hi_constr);
}

pub(crate) fn eval_shift_left<F: Field, P: PackedField<Scalar = F>>(
    lv: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_shl = lv[IS_SHIFT_LEFT];
    let one = P::ONES;

    let (delta_ge32, lo_shifted_0, lo_shifted_1, hi_shifted_0, _hi_shifted_1, output_lo, output_hi) =
        eval_rotate_shift(lv, yield_constr, is_shl);

    // Intuitively we want to do this (which works when delta <= 32):
    //let lo_constr = lo_shifted_0 - output_lo;
    //let hi_constr = lo_shifted_1 + hi_shifted_0 - output_hi;

    let lo_constr =
        (one - delta_ge32) * (lo_shifted_0 - output_lo) + delta_ge32 * (P::ZEROS - output_lo);
    let hi_constr = (one - delta_ge32) * (lo_shifted_1 + hi_shifted_0 - output_hi)
        + delta_ge32 * (lo_shifted_0 - output_hi);
    yield_constr.constraint(is_shl * lo_constr);
    yield_constr.constraint(is_shl * hi_constr);
}

pub(crate) fn eval_shift_right<F: Field, P: PackedField<Scalar = F>>(
    lv: &[P; NUM_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let is_shl = lv[IS_SHIFT_LEFT];
    let one = P::ONES;

    let (delta_ge32, _lo_shifted_0, lo_shifted_1, hi_shifted_0, hi_shifted_1, output_lo, output_hi) =
        eval_rotate_shift(lv, yield_constr, is_shl);

    // Intuitively we want to do this (which works when delta <= 32):
    //let lo_constr = lo_shifted_1 + hi_shifted_0 - output_hi;
    //let hi_constr = hi_shifted_1 - output_lo;

    let lo_constr = (one - delta_ge32) * (lo_shifted_1 + hi_shifted_0 - output_lo)
        + delta_ge32 * (hi_shifted_1 - output_lo);
    let hi_constr =
        (one - delta_ge32) * (hi_shifted_1 - output_hi) + delta_ge32 * (P::ZEROS - output_hi);
    yield_constr.constraint(is_shl * lo_constr);
    yield_constr.constraint(is_shl * hi_constr);
}

fn constrain_pow_exp_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    filter: ExtensionTarget<D>,
) {
    let exp_bits = COL_ROTATE_SHIFT_EXP_BITS.map(|r| lv[r]);
    let exp_ge32_bit = lv[COL_ROTATE_SHIFT_DELTA_DIV32];

    let pow_exp_aux_0 = lv[COL_ROTATE_SHIFT_POW_EXP_AUX_0];
    let pow_exp_aux_1 = lv[COL_ROTATE_SHIFT_POW_EXP_AUX_1];
    let pow_exp_aux_2 = lv[COL_ROTATE_SHIFT_POW_EXP_AUX_2];
    let pow_exp = lv[COL_ROTATE_SHIFT_POW_EXP];

    // Check that every "bit" of exp_bits and exp_ge32_bit is 0 or 1
    constrain_all_to_bits_circuit(builder, yield_constr, filter, &exp_bits);
    constrain_all_to_bits_circuit(builder, yield_constr, filter, &[exp_ge32_bit]);

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
        let t1 = builder.mul_sub_extension(pow_exp_aux_2, t0, pow_exp);
        builder.mul_extension(filter, t1)
    };
    yield_constr.constraint(builder, constr4);
}

fn constrain_shifted_are_valid_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    filter: ExtensionTarget<D>,
) {
    let lo_shifted_0 = lv[COL_ROTATE_SHIFT_INPUT_LO_DISPLACED_0];
    let lo_shifted_1 = lv[COL_ROTATE_SHIFT_INPUT_LO_DISPLACED_1];
    let hi_shifted_0 = lv[COL_ROTATE_SHIFT_INPUT_HI_DISPLACED_0];
    let hi_shifted_1 = lv[COL_ROTATE_SHIFT_INPUT_HI_DISPLACED_1];
    let lo_shifted_aux_0 = lv[COL_ROTATE_SHIFT_INPUT_LO_DISPLACED_AUX_0];
    let lo_shifted_aux_1 = lv[COL_ROTATE_SHIFT_INPUT_LO_DISPLACED_AUX_1];
    let hi_shifted_aux_0 = lv[COL_ROTATE_SHIFT_INPUT_HI_DISPLACED_AUX_0];
    let hi_shifted_aux_1 = lv[COL_ROTATE_SHIFT_INPUT_HI_DISPLACED_AUX_1];

    let one = builder.one_extension();
    let u32_max = builder.constant_extension(F::Extension::from_canonical_u32(u32::MAX));

    let constr1 = {
        let t0 = builder.sub_extension(u32_max, lo_shifted_1);
        let t1 = builder.mul_sub_extension(lo_shifted_aux_0, t0, lo_shifted_aux_1);
        builder.mul_extension(filter, t1)
    };
    yield_constr.constraint(builder, constr1);

    let constr2 = {
        let t0 = builder.sub_extension(u32_max, hi_shifted_1);
        let t1 = builder.mul_sub_extension(hi_shifted_aux_0, t0, hi_shifted_aux_1);
        builder.mul_extension(filter, t1)
    };
    yield_constr.constraint(builder, constr2);

    let lo_shifted_is_valid = {
        let t0 = builder.sub_extension(one, lo_shifted_aux_1);
        let t1 = builder.mul_extension(t0, lo_shifted_0);
        builder.mul_extension(filter, t1)
    };
    let hi_shifted_is_valid = {
        let t0 = builder.sub_extension(one, hi_shifted_aux_1);
        let t1 = builder.mul_extension(t0, hi_shifted_0);
        builder.mul_extension(filter, t1)
    };
    yield_constr.constraint(builder, lo_shifted_is_valid);
    yield_constr.constraint(builder, hi_shifted_is_valid);
}

fn eval_rotate_shift_circuit<F: RichField + Extendable<D>, const D: usize>(
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

    let exp_ge32_bit = lv[COL_ROTATE_SHIFT_DELTA_DIV32];
    let pow_exp = lv[COL_ROTATE_SHIFT_POW_EXP];

    let lo_shifted_0 = lv[COL_ROTATE_SHIFT_INPUT_LO_DISPLACED_0];
    let lo_shifted_1 = lv[COL_ROTATE_SHIFT_INPUT_LO_DISPLACED_1];
    let hi_shifted_0 = lv[COL_ROTATE_SHIFT_INPUT_HI_DISPLACED_0];
    let hi_shifted_1 = lv[COL_ROTATE_SHIFT_INPUT_HI_DISPLACED_1];

    // Output
    let output_lo = lv[COL_ROTATE_SHIFT_OUTPUT_0];
    let output_hi = lv[COL_ROTATE_SHIFT_OUTPUT_1];

    constrain_pow_exp_circuit(builder, lv, yield_constr, filter);
    constrain_shifted_are_valid_circuit(builder, lv, yield_constr, filter);

    let base = builder.constant_extension(F::Extension::from_canonical_u64(1u64 << 32));
    let lo_shifted = builder.mul_add_extension(lo_shifted_1, base, lo_shifted_0);
    let hi_shifted = builder.mul_add_extension(hi_shifted_1, base, hi_shifted_0);

    let lo_shifted_expected = builder.mul_extension(input_lo, pow_exp);
    let hi_shifted_expected = builder.mul_extension(input_hi, pow_exp);

    let lo_shifted_valid = {
        let t0 = builder.sub_extension(lo_shifted_expected, lo_shifted);
        builder.mul_extension(filter, t0)
    };
    yield_constr.constraint(builder, lo_shifted_valid);
    let hi_shifted_valid = {
        let t0 = builder.sub_extension(hi_shifted_expected, hi_shifted);
        builder.mul_extension(filter, t0)
    };
    yield_constr.constraint(builder, hi_shifted_valid);

    (
        exp_ge32_bit,
        lo_shifted_0,
        lo_shifted_1,
        hi_shifted_0,
        hi_shifted_1,
        output_lo,
        output_hi,
    )
}

pub(crate) fn eval_rotate_left_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_rol = lv[IS_ROTATE_LEFT];

    let (delta_ge32, lo_shifted_0, lo_shifted_1, hi_shifted_0, hi_shifted_1, output_lo, output_hi) =
        eval_rotate_shift_circuit(builder, lv, yield_constr, is_rol);

    let one = builder.one_extension();
    let s0 = builder.add_extension(hi_shifted_1, lo_shifted_0);
    let s1 = builder.add_extension(lo_shifted_1, hi_shifted_0);
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

pub(crate) fn eval_rotate_right_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_ror = lv[IS_ROTATE_RIGHT];

    let (delta_ge32, lo_shifted_0, lo_shifted_1, hi_shifted_0, hi_shifted_1, output_lo, output_hi) =
        eval_rotate_shift_circuit(builder, lv, yield_constr, is_ror);

    let one = builder.one_extension();
    let s0 = builder.add_extension(hi_shifted_1, lo_shifted_0);
    let s1 = builder.add_extension(lo_shifted_1, hi_shifted_0);
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

pub(crate) fn eval_shift_left_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_shl = lv[IS_SHIFT_LEFT];

    let (delta_ge32, lo_shifted_0, lo_shifted_1, hi_shifted_0, _hi_shifted_1, output_lo, output_hi) =
        eval_rotate_shift_circuit(builder, lv, yield_constr, is_shl);

    let one = builder.one_extension();
    let c = builder.sub_extension(one, delta_ge32);

    let lo_constr = {
        let t0 = builder.sub_extension(lo_shifted_0, output_lo);
        let t1 = builder.mul_extension(c, t0);
        let t2 = builder.mul_extension(delta_ge32, output_lo);
        let t3 = builder.add_extension(t1, t2);
        builder.mul_extension(is_shl, t3)
    };

    let hi_constr = {
        let t0 = builder.add_extension(lo_shifted_1, hi_shifted_0);
        let t1 = builder.sub_extension(t0, output_hi);
        let t2 = builder.mul_extension(c, t1);
        let t3 = builder.sub_extension(lo_shifted_0, output_hi);
        let t4 = builder.mul_extension(delta_ge32, t3);
        let t5 = builder.add_extension(t2, t4);
        builder.mul_extension(is_shl, t5)
    };

    yield_constr.constraint(builder, lo_constr);
    yield_constr.constraint(builder, hi_constr);
}

pub(crate) fn eval_shift_right_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let is_shr = lv[IS_SHIFT_RIGHT];

    let (delta_ge32, _lo_shifted_0, lo_shifted_1, hi_shifted_0, hi_shifted_1, output_lo, output_hi) =
        eval_rotate_shift_circuit(builder, lv, yield_constr, is_shr);

    let one = builder.one_extension();
    let c = builder.sub_extension(one, delta_ge32);

    let lo_constr = {
        let t0 = builder.add_extension(lo_shifted_1, hi_shifted_0);
        let t1 = builder.sub_extension(t0, output_lo);
        let t2 = builder.mul_extension(c, t1);
        let t3 = builder.sub_extension(hi_shifted_1, output_lo);
        let t4 = builder.mul_extension(delta_ge32, t3);
        let t5 = builder.add_extension(t2, t4);
        builder.mul_extension(is_shr, t5)
    };

    let hi_constr = {
        let t0 = builder.sub_extension(hi_shifted_1, output_hi);
        let t1 = builder.mul_extension(c, t0);
        let t2 = builder.mul_extension(delta_ge32, output_hi);
        let t3 = builder.add_extension(t1, t2);
        builder.mul_extension(is_shr, t3)
    };

    yield_constr.constraint(builder, lo_constr);
    yield_constr.constraint(builder, hi_constr);
}
