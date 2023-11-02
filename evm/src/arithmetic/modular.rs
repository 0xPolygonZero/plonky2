//! Support for the EVM modular instructions ADDMOD, SUBMOD, MULMOD and MOD,
//! as well as DIV and FP254 related modular instructions.
//!
//! This crate verifies an EVM modular instruction, which takes three
//! 256-bit inputs A, B and M, and produces a 256-bit output C satisfying
//!
//!    C = operation(A, B) (mod M).
//!
//! where operation can be addition, multiplication, or just return
//! the first argument (for MOD).  Inputs A, B and M, and output C,
//! are given as arrays of 16-bit limbs. For example, if the limbs of
//! A are a[0]...a[15], then
//!
//!    A = \sum_{i=0}^15 a[i] β^i,
//!
//! where β = 2^16 = 2^LIMB_BITS. To verify that A, B, M and C satisfy
//! the equation we proceed as follows. Define
//!
//!    a(x) = \sum_{i=0}^15 a[i] x^i
//!
//! (so A = a(β)) and similarly for b(x), m(x) and c(x). Then
//! operation(A,B) = C (mod M) if and only if there exists q such that
//! the polynomial
//!
//!    operation(a(x), b(x)) - c(x) - m(x) * q(x)
//!
//! is zero when evaluated at x = β, i.e. it is divisible by (x - β);
//! equivalently, there exists a polynomial s such that
//!
//!    operation(a(x), b(x)) - c(x) - m(x) * q(x) - (x - β) * s(x) == 0
//!
//! if and only if operation(A,B) = C (mod M). In the code below, this
//! "constraint polynomial" is constructed in the variable
//! `constr_poly`. It must be identically zero for the modular
//! operation to be verified, or, equivalently, each of its
//! coefficients must be zero. The variable names of the constituent
//! polynomials are (writing N for N_LIMBS=16):
//!
//!   a(x) = \sum_{i=0}^{N-1} input0[i] * x^i
//!   b(x) = \sum_{i=0}^{N-1} input1[i] * x^i
//!   c(x) = \sum_{i=0}^{N-1} output[i] * x^i
//!   m(x) = \sum_{i=0}^{N-1} modulus[i] * x^i
//!   q(x) = \sum_{i=0}^{2N-1} quot[i] * x^i
//!   s(x) = \sum_i^{2N-2} aux[i] * x^i
//!
//! Because A, B, M and C are 256-bit numbers, the degrees of a, b, m
//! and c are (at most) N-1 = 15. If m = 1, then Q would be A*B which
//! can be up to 2^512 - ε, so deg(q) can be up to 2*N-1 = 31. Note
//! that, although for arbitrary m and q we might have deg(m*q) = 3*N-2,
//! because the magnitude of M*Q must match that of operation(A,B), we
//! always have deg(m*q) <= 2*N-1. Finally, in order for all the degrees
//! to match, we have deg(s) <= 2*N-2 = 30.
//!
//! -*-
//!
//! To verify that the output is reduced, that is, output < modulus,
//! the prover supplies the value `out_aux_red` which must satisfy
//!
//!    output - modulus = out_aux_red + 2^256
//!
//! and these values are passed to the "less than" operation.
//!
//! -*-
//!
//! The EVM defines division by zero as zero. We handle this as
//! follows:
//!
//! The prover supplies a binary value `mod_is_zero` which is one if
//! the modulus is zero and zero otherwise. This is verified, then
//! added to the modulus (this can't overflow, as modulus[0] was
//! range-checked and mod_is_zero is 0 or 1). The rest of the
//! calculation proceeds as if modulus was actually 1; this correctly
//! verifies that the output is zero, as required by the standard.
//! To summarise:
//!
//! - mod_is_zero is 0 or 1
//! - if mod_is_zero is 1, then
//!    - given modulus is 0
//!    - updated modulus is 1, which forces the correct output of 0
//! - if mod_is_zero is 0, then
//!    - given modulus can be 0 or non-zero
//!    - updated modulus is same as given
//!    - if modulus is non-zero, correct output is obtained
//!    - if modulus is 0, then the test output < modulus, checking that
//!      the output is reduced, will fail, because output is non-negative.
//!
//! In the case of DIV, we do something similar, except that we "replace"
//! the modulus with "2^256" to force the quotient to be zero.
//!
//! -*-
//!
//! NB: The implementation uses 9 * N_LIMBS = 144 columns because of
//! the requirements of the general purpose MULMOD; since ADDMOD,
//! SUBMOD, MOD and DIV are currently implemented in terms of the
//! general modular code, they also take 144 columns. Possible
//! improvements:
//!
//! - We could reduce the number of columns to 112 for ADDMOD, SUBMOD,
//!   etc. if they were implemented separately, so they don't pay the
//!   full cost of the general MULMOD.
//!
//! - All these operations could have alternative forms where the
//!   output was not guaranteed to be reduced, which is often sufficient
//!   in practice, and which would save a further 16 columns.
//!
//! - If the modulus is known in advance (such as for elliptic curve
//!   arithmetic), specialised handling of MULMOD in that case would
//!   only require 96 columns, or 80 if the output doesn't need to be
//!   reduced.

use std::ops::Range;

use ethereum_types::U256;
use num::bigint::Sign;
use num::{BigInt, One, Zero};
use plonky2::field::extension::Extendable;
use plonky2::field::packed::PackedField;
use plonky2::field::types::{Field, PrimeField64};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::ext_target::ExtensionTarget;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use static_assertions::const_assert;

use super::columns;
use crate::arithmetic::addcy::{eval_ext_circuit_addcy, eval_packed_generic_addcy};
use crate::arithmetic::columns::*;
use crate::arithmetic::utils::*;
use crate::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use crate::extension_tower::BN_BASE;

const fn bn254_modulus_limbs() -> [u16; N_LIMBS] {
    const_assert!(N_LIMBS == 16); // Assumed below
    let mut limbs = [0u16; N_LIMBS];
    let mut i = 0;
    while i < N_LIMBS / 4 {
        let x = BN_BASE.0[i];
        limbs[4 * i] = x as u16;
        limbs[4 * i + 1] = (x >> 16) as u16;
        limbs[4 * i + 2] = (x >> 32) as u16;
        limbs[4 * i + 3] = (x >> 48) as u16;
        i += 1;
    }
    limbs
}

/// Convert the base-2^16 representation of a number into a BigInt.
///
/// Given `N` signed (16 + ε)-bit values in `limbs`, return the BigInt
///
///   \sum_{i=0}^{N-1} limbs[i] * β^i.
///
/// This is basically "evaluate the given polynomial at β". Although
/// the input type is i64, the values must always be in (-2^16 - ε,
/// 2^16 + ε) because of the caller's range check on the inputs (the ε
/// allows us to convert calculated output, which can be bigger than
/// 2^16).
fn columns_to_bigint<const N: usize>(limbs: &[i64; N]) -> BigInt {
    const BASE: i64 = 1i64 << LIMB_BITS;

    let mut pos_limbs_u32 = Vec::with_capacity(N / 2 + 1);
    let mut neg_limbs_u32 = Vec::with_capacity(N / 2 + 1);
    let mut cy = 0i64; // cy is necessary to handle ε > 0
    for i in 0..(N / 2) {
        let t = cy + limbs[2 * i] + BASE * limbs[2 * i + 1];
        pos_limbs_u32.push(if t > 0 { t as u32 } else { 0u32 });
        neg_limbs_u32.push(if t < 0 { -t as u32 } else { 0u32 });
        cy = t / (1i64 << 32);
    }
    if N & 1 != 0 {
        // If N is odd we need to add the last limb on its own
        let t = cy + limbs[N - 1];
        pos_limbs_u32.push(if t > 0 { t as u32 } else { 0u32 });
        neg_limbs_u32.push(if t < 0 { -t as u32 } else { 0u32 });
        cy = t / (1i64 << 32);
    }
    pos_limbs_u32.push(if cy > 0 { cy as u32 } else { 0u32 });
    neg_limbs_u32.push(if cy < 0 { -cy as u32 } else { 0u32 });

    let pos = BigInt::from_slice(Sign::Plus, &pos_limbs_u32);
    let neg = BigInt::from_slice(Sign::Plus, &neg_limbs_u32);
    pos - neg
}

/// Convert a BigInt into a base-2^16 representation.
///
/// Given a BigInt `num`, return an array of `N` signed 16-bit
/// values, say `limbs`, such that
///
///   num = \sum_{i=0}^{N-1} limbs[i] * β^i.
///
/// Note that `N` must be at least ceil(log2(num)/16) in order to be
/// big enough to hold `num`.
fn bigint_to_columns<const N: usize>(num: &BigInt) -> [i64; N] {
    assert!(num.bits() <= 16 * N as u64);
    let mut output = [0i64; N];
    for (i, limb) in num.iter_u32_digits().enumerate() {
        output[2 * i] = limb as u16 as i64;
        output[2 * i + 1] = (limb >> LIMB_BITS) as i64;
    }
    if num.sign() == Sign::Minus {
        for c in output.iter_mut() {
            *c = -*c;
        }
    }
    output
}

/// Generate the output and auxiliary values for given `operation`.
///
/// NB: `operation` can set the higher order elements in its result to
/// zero if they are not used.
pub(crate) fn generate_modular_op<F: PrimeField64>(
    lv: &[F],
    nv: &mut [F],
    filter: usize,
    pol_input: [i64; 2 * N_LIMBS - 1],
    modulus_range: Range<usize>,
) -> ([F; N_LIMBS], [F; 2 * N_LIMBS]) {
    assert!(modulus_range.len() == N_LIMBS);
    let mut modulus_limbs = read_value_i64_limbs(lv, modulus_range);

    // BigInts are just used to avoid having to implement modular
    // reduction.
    let mut modulus = columns_to_bigint(&modulus_limbs);

    // constr_poly is initialised to the input calculation as
    // polynomials, and is used as such for the BigInt reduction;
    // later, other values are added/subtracted, which is where its
    // meaning as the "constraint polynomial" comes in.
    let mut constr_poly = [0i64; 2 * N_LIMBS];
    constr_poly[..2 * N_LIMBS - 1].copy_from_slice(&pol_input);

    // two_exp_256 == 2^256
    let two_exp_256 = {
        let mut t = BigInt::zero();
        t.set_bit(256, true);
        t
    };

    let mut mod_is_zero = F::ZERO;
    if modulus.is_zero() {
        if filter == columns::IS_DIV || filter == columns::IS_SHR {
            // set modulus = 2^256; the condition above means we know
            // it's zero at this point, so we can just set bit 256.
            modulus.set_bit(256, true);
            // modulus_limbs don't play a role below
        } else {
            // set modulus = 1
            modulus = BigInt::one();
            modulus_limbs[0] = 1i64;
        }
        mod_is_zero = F::ONE;
    }

    let input = columns_to_bigint(&constr_poly);

    // modulus != 0 here, because, if the given modulus was zero, then
    // it was set to 1 or 2^256 above
    let mut output = &input % &modulus;
    // output will be -ve (but > -modulus) if input was -ve, so we can
    // add modulus to obtain a "canonical" +ve output.
    if output.sign() == Sign::Minus {
        output += &modulus;
    }
    let output_limbs = bigint_to_columns::<N_LIMBS>(&output);
    // exact division; can be -ve for SUB* operations.
    let quot = (&input - &output) / &modulus;
    if quot.sign() == Sign::Minus {
        debug_assert!(filter == IS_SUBMOD || filter == IS_SUBFP254);
    }
    let mut quot_limbs = bigint_to_columns::<{ 2 * N_LIMBS }>(&quot);

    // output < modulus here; the proof requires (output - modulus) % 2^256:
    let out_aux_red = bigint_to_columns::<N_LIMBS>(&(two_exp_256 - modulus + output));

    // constr_poly is the array of coefficients of the polynomial
    //
    //   operation(a(x), b(x)) - c(x) - s(x)*m(x).
    //
    pol_sub_assign(&mut constr_poly, &output_limbs);
    let prod = pol_mul_wide2(quot_limbs, modulus_limbs);
    pol_sub_assign(&mut constr_poly, &prod[0..2 * N_LIMBS]);

    // Higher order terms of the product must be zero for valid quot and modulus:
    debug_assert!(&prod[2 * N_LIMBS..].iter().all(|&x| x == 0i64));

    // constr_poly must be zero when evaluated at x = β :=
    // 2^LIMB_BITS, hence it's divisible by (x - β). `aux_limbs` is
    // the result of removing that root.
    let mut aux_limbs = pol_remove_root_2exp::<LIMB_BITS, _, { 2 * N_LIMBS }>(constr_poly);

    for c in aux_limbs.iter_mut() {
        // we store the unsigned offset value c + 2^20.
        *c += AUX_COEFF_ABS_MAX;
    }
    debug_assert!(aux_limbs.iter().all(|&c| c.abs() <= 2 * AUX_COEFF_ABS_MAX));

    for (i, &c) in MODULAR_AUX_INPUT_LO.zip(&aux_limbs[..2 * N_LIMBS - 1]) {
        nv[i] = F::from_canonical_u16(c as u16);
    }
    for (i, &c) in MODULAR_AUX_INPUT_HI.zip(&aux_limbs[..2 * N_LIMBS - 1]) {
        nv[i] = F::from_canonical_u16((c >> 16) as u16);
    }

    // quo_input can be negative for SUB* operations, so we offset it
    // to ensure it's positive.
    if [columns::IS_SUBMOD, columns::IS_SUBFP254].contains(&filter) {
        let (lo, hi) = quot_limbs.split_at_mut(N_LIMBS);

        // Verify that the elements are in the expected range.
        debug_assert!(lo.iter().all(|&c| c <= u16::max_value() as i64));

        // Top half of quot_limbs should be zero.
        debug_assert!(hi.iter().all(|&d| d.is_zero()));

        if quot.sign() == Sign::Minus {
            // quot is negative, so each c should be negative, i.e. in
            // the range [-(2^16 - 1), 0]; so we add 2^16 - 1 to c so
            // it's in the range [0, 2^16 - 1] which will correctly
            // range-check.
            for c in lo {
                *c += u16::max_value() as i64;
            }
            // Store the sign of the quotient after the quotient.
            hi[0] = 1;
        } else {
            hi[0] = 0;
        };
    }

    nv[MODULAR_MOD_IS_ZERO] = mod_is_zero;
    nv[MODULAR_OUT_AUX_RED].copy_from_slice(&out_aux_red.map(F::from_canonical_i64));
    nv[MODULAR_DIV_DENOM_IS_ZERO] = mod_is_zero * (lv[IS_DIV] + lv[IS_SHR]);

    (
        output_limbs.map(F::from_canonical_i64),
        quot_limbs.map(F::from_noncanonical_i64),
    )
}

/// Generate the output and auxiliary values for modular operations.
///
/// `filter` must be one of `columns::IS_{ADD,MUL,SUB}{MOD,FP254}`.
pub(crate) fn generate<F: PrimeField64>(
    lv: &mut [F],
    nv: &mut [F],
    filter: usize,
    input0: U256,
    input1: U256,
    modulus: U256,
) {
    debug_assert!(lv.len() == NUM_ARITH_COLUMNS && nv.len() == NUM_ARITH_COLUMNS);

    u256_to_array(&mut lv[MODULAR_INPUT_0], input0);
    u256_to_array(&mut lv[MODULAR_INPUT_1], input1);
    u256_to_array(&mut lv[MODULAR_MODULUS], modulus);

    if [
        columns::IS_ADDFP254,
        columns::IS_SUBFP254,
        columns::IS_MULFP254,
    ]
    .contains(&filter)
    {
        debug_assert!(modulus == BN_BASE);
    }

    // Inputs are all in [0, 2^16), so the "as i64" conversion is safe.
    let input0_limbs = read_value_i64_limbs(lv, MODULAR_INPUT_0);
    let input1_limbs = read_value_i64_limbs(lv, MODULAR_INPUT_1);

    let pol_input = match filter {
        columns::IS_ADDMOD | columns::IS_ADDFP254 => pol_add(input0_limbs, input1_limbs),
        columns::IS_SUBMOD | columns::IS_SUBFP254 => pol_sub(input0_limbs, input1_limbs),
        columns::IS_MULMOD | columns::IS_MULFP254 => pol_mul_wide(input0_limbs, input1_limbs),
        _ => panic!("generate modular operation called with unknown opcode"),
    };
    let (out, quo_input) = generate_modular_op(lv, nv, filter, pol_input, MODULAR_MODULUS);
    lv[MODULAR_OUTPUT].copy_from_slice(&out);
    lv[MODULAR_QUO_INPUT].copy_from_slice(&quo_input);
}

pub(crate) fn check_reduced<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    nv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
    filter: P,
    output: [P; N_LIMBS],
    modulus: [P; N_LIMBS],
    mod_is_zero: P,
) {
    // Verify that the output is reduced, i.e. output < modulus.
    let out_aux_red = &nv[MODULAR_OUT_AUX_RED];
    // This sets is_less_than to 1 unless we get mod_is_zero when
    // doing a DIV or SHR; in that case, we need is_less_than=0, since
    // eval_packed_generic_addcy checks
    //
    //   modulus + out_aux_red == output + is_less_than*2^256
    //
    // and we are given output = out_aux_red when modulus is zero.
    let mut is_less_than = [P::ZEROS; N_LIMBS];
    is_less_than[0] = P::ONES - mod_is_zero * (lv[IS_DIV] + lv[IS_SHR]);
    // NB: output and modulus in lv while out_aux_red and
    // is_less_than (via mod_is_zero) depend on nv, hence the
    // 'is_two_row_op' argument is set to 'true'.
    eval_packed_generic_addcy(
        yield_constr,
        filter,
        &modulus,
        out_aux_red,
        &output,
        &is_less_than,
        true,
    );
}

/// Build the part of the constraint polynomial that applies to the
/// DIV, MOD, ADDMOD, MULMOD operations (and the FP254 variants), and
/// perform the common verifications.
///
/// Specifically, with the notation above, build the polynomial
///
///   c(x) + q(x) * m(x) + (x - β) * s(x)
///
/// and check consistency when m = 0, and that c is reduced. Note that
/// q(x) CANNOT be negative here, but, in contrast to
/// addsubmod_constr_poly above, it is twice as long.
pub(crate) fn modular_constr_poly<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    nv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
    filter: P,
    mut output: [P; N_LIMBS],
    mut modulus: [P; N_LIMBS],
    quot: [P; 2 * N_LIMBS],
) -> [P; 2 * N_LIMBS] {
    let mod_is_zero = nv[MODULAR_MOD_IS_ZERO];

    // Check that mod_is_zero is zero or one
    yield_constr.constraint_transition(filter * (mod_is_zero * mod_is_zero - mod_is_zero));

    // Check that mod_is_zero is zero if modulus is not zero (they
    // could both be zero)
    let limb_sum = modulus.into_iter().sum::<P>();
    yield_constr.constraint_transition(filter * limb_sum * mod_is_zero);

    // See the file documentation for why this suffices to handle
    // modulus = 0.
    modulus[0] += mod_is_zero;

    // Is 1 iff the operation is DIV or SHR and the denominator is zero.
    let div_denom_is_zero = nv[MODULAR_DIV_DENOM_IS_ZERO];
    yield_constr.constraint_transition(
        filter * (mod_is_zero * (lv[IS_DIV] + lv[IS_SHR]) - div_denom_is_zero),
    );

    // Needed to compensate for adding mod_is_zero to modulus above,
    // since the call eval_packed_generic_addcy() below subtracts modulus
    // to verify in the case of a DIV or SHR.
    output[0] += div_denom_is_zero;

    check_reduced(lv, nv, yield_constr, filter, output, modulus, mod_is_zero);

    // restore output[0]
    output[0] -= div_denom_is_zero;

    // prod = q(x) * m(x)
    let prod = pol_mul_wide2(quot, modulus);
    // higher order terms must be zero
    for &x in prod[2 * N_LIMBS..].iter() {
        yield_constr.constraint_transition(filter * x);
    }

    // constr_poly = c(x) + q(x) * m(x)
    let mut constr_poly: [_; 2 * N_LIMBS] = prod[0..2 * N_LIMBS].try_into().unwrap();
    pol_add_assign(&mut constr_poly, &output);

    let base = P::Scalar::from_canonical_u64(1 << LIMB_BITS);
    let offset = P::Scalar::from_canonical_u64(AUX_COEFF_ABS_MAX as u64);

    // constr_poly = c(x) + q(x) * m(x) + (x - β) * s(x)c
    let mut aux = [P::ZEROS; 2 * N_LIMBS];
    for (c, i) in aux.iter_mut().zip(MODULAR_AUX_INPUT_LO) {
        // MODULAR_AUX_INPUT elements were offset by 2^20 in
        // generation, so we undo that here.
        *c = nv[i] - offset;
    }
    // add high 16-bits of aux input
    for (c, j) in aux.iter_mut().zip(MODULAR_AUX_INPUT_HI) {
        *c += base * nv[j];
    }

    pol_add_assign(&mut constr_poly, &pol_adjoin_root(aux, base));

    constr_poly
}

/// Build the part of the constraint polynomial that's common to the
/// SUBMOD and SUBFP254 operations, and perform the common
/// verifications.
///
/// Specifically, with the notation above, build the polynomial
///
///   c(x) + q(x) * m(x) + (x - β) * s(x)
///
/// and check consistency when m = 0, and that c is reduced. Note that
/// q(x) can be negative here, so it needs to be reconstructed from
/// its hi and lo halves in MODULAR_QUO_INPUT and then to be
/// "de-biassed" from the range [0, 2^32) to the correct range
/// (-2^16,2^16).
pub(crate) fn submod_constr_poly<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    nv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
    filter: P,
    output: [P; N_LIMBS],
    modulus: [P; N_LIMBS],
    mut quot: [P; 2 * N_LIMBS],
) -> [P; 2 * N_LIMBS] {
    // quot was offset by 2^16 - 1 if it was negative; we undo that
    // offset here:
    let (lo, hi) = quot.split_at_mut(N_LIMBS);
    let sign = hi[0];
    // sign must be 1 (negative) or 0 (positive)
    yield_constr.constraint(filter * sign * (sign - P::ONES));
    let offset = P::Scalar::from_canonical_u16(u16::max_value());
    for c in lo {
        *c -= offset * sign;
    }
    hi[0] = P::ZEROS;
    for d in hi {
        // All higher limbs must be zero
        yield_constr.constraint(filter * *d);
    }

    modular_constr_poly(lv, nv, yield_constr, filter, output, modulus, quot)
}

/// Add constraints for modular operations.
pub(crate) fn eval_packed<P: PackedField>(
    lv: &[P; NUM_ARITH_COLUMNS],
    nv: &[P; NUM_ARITH_COLUMNS],
    yield_constr: &mut ConstraintConsumer<P>,
) {
    // NB: The CTL code guarantees that filter is 0 or 1, i.e. that
    // only one of the operations below is "live".
    let bn254_filter =
        lv[columns::IS_ADDFP254] + lv[columns::IS_MULFP254] + lv[columns::IS_SUBFP254];
    let filter =
        lv[columns::IS_ADDMOD] + lv[columns::IS_SUBMOD] + lv[columns::IS_MULMOD] + bn254_filter;

    // Ensure that this operation is not the last row of the table;
    // needed because we access the next row of the table in nv.
    yield_constr.constraint_last_row(filter);

    // Verify that the modulus is the BN254 modulus for the
    // {ADD,MUL,SUB}FP254 operations.
    let modulus = read_value::<N_LIMBS, _>(lv, MODULAR_MODULUS);
    for (&mi, bi) in modulus.iter().zip(bn254_modulus_limbs()) {
        yield_constr.constraint_transition(bn254_filter * (mi - P::Scalar::from_canonical_u16(bi)));
    }

    let output = read_value::<N_LIMBS, _>(lv, MODULAR_OUTPUT);
    let quo_input = read_value::<{ 2 * N_LIMBS }, _>(lv, MODULAR_QUO_INPUT);

    let add_filter = lv[columns::IS_ADDMOD] + lv[columns::IS_ADDFP254];
    let sub_filter = lv[columns::IS_SUBMOD] + lv[columns::IS_SUBFP254];
    let mul_filter = lv[columns::IS_MULMOD] + lv[columns::IS_MULFP254];
    let addmul_filter = add_filter + mul_filter;

    // constr_poly has 2*N_LIMBS limbs
    let submod_constr_poly =
        submod_constr_poly(lv, nv, yield_constr, sub_filter, output, modulus, quo_input);
    let modular_constr_poly = modular_constr_poly(
        lv,
        nv,
        yield_constr,
        addmul_filter,
        output,
        modulus,
        quo_input,
    );

    let input0 = read_value(lv, MODULAR_INPUT_0);
    let input1 = read_value(lv, MODULAR_INPUT_1);

    let add_input = pol_add(input0, input1);
    let sub_input = pol_sub(input0, input1);
    let mul_input = pol_mul_wide(input0, input1);

    for (input, &filter, constr_poly) in [
        (&add_input, &add_filter, modular_constr_poly),
        (&sub_input, &sub_filter, submod_constr_poly),
        (&mul_input, &mul_filter, modular_constr_poly),
    ] {
        // Need constr_poly_copy to be the first argument to
        // pol_sub_assign, since it is the longer of the two
        // arguments.
        let mut constr_poly_copy = constr_poly;
        pol_sub_assign(&mut constr_poly_copy, input);

        // At this point constr_poly_copy holds the coefficients of
        // the polynomial
        //
        //   operation(a(x), b(x)) - c(x) - q(x) * m(x) - (x - β) * s(x)
        //
        // where operation is add, mul or |a,b|->a.  The modular
        // operation is valid if and only if all of those coefficients
        // are zero.
        for &c in constr_poly_copy.iter() {
            yield_constr.constraint_transition(filter * c);
        }
    }
}

pub(crate) fn modular_constr_poly_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    nv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    builder: &mut CircuitBuilder<F, D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    filter: ExtensionTarget<D>,
    mut output: [ExtensionTarget<D>; N_LIMBS],
    mut modulus: [ExtensionTarget<D>; N_LIMBS],
    quot: [ExtensionTarget<D>; 2 * N_LIMBS],
) -> [ExtensionTarget<D>; 2 * N_LIMBS] {
    let mod_is_zero = nv[MODULAR_MOD_IS_ZERO];

    // Check that mod_is_zero is zero or one
    let t = builder.mul_sub_extension(mod_is_zero, mod_is_zero, mod_is_zero);
    let t = builder.mul_extension(filter, t);
    yield_constr.constraint_transition(builder, t);

    // Check that mod_is_zero is zero if modulus is not zero (they
    // could both be zero)
    let limb_sum = builder.add_many_extension(modulus);
    let t = builder.mul_extension(limb_sum, mod_is_zero);
    let t = builder.mul_extension(filter, t);
    yield_constr.constraint_transition(builder, t);

    modulus[0] = builder.add_extension(modulus[0], mod_is_zero);

    // Is 1 iff the operation is DIV or SHR and the denominator is zero.
    let div_denom_is_zero = nv[MODULAR_DIV_DENOM_IS_ZERO];
    let div_shr_filter = builder.add_extension(lv[IS_DIV], lv[IS_SHR]);
    let t = builder.mul_sub_extension(mod_is_zero, div_shr_filter, div_denom_is_zero);
    let t = builder.mul_extension(filter, t);
    yield_constr.constraint_transition(builder, t);

    // Needed to compensate for adding mod_is_zero to modulus above,
    // since the call eval_packed_generic_addcy() below subtracts modulus
    // to verify in the case of a DIV or SHR.
    output[0] = builder.add_extension(output[0], div_denom_is_zero);

    // Verify that the output is reduced, i.e. output < modulus.
    let out_aux_red = &nv[MODULAR_OUT_AUX_RED];
    let one = builder.one_extension();
    let zero = builder.zero_extension();
    let mut is_less_than = [zero; N_LIMBS];
    is_less_than[0] =
        builder.arithmetic_extension(F::NEG_ONE, F::ONE, mod_is_zero, div_shr_filter, one);

    eval_ext_circuit_addcy(
        builder,
        yield_constr,
        filter,
        &modulus,
        out_aux_red,
        &output,
        &is_less_than,
        true,
    );
    // restore output[0]
    output[0] = builder.sub_extension(output[0], div_denom_is_zero);

    // prod = q(x) * m(x)
    let prod = pol_mul_wide2_ext_circuit(builder, quot, modulus);
    // higher order terms must be zero
    for &x in prod[2 * N_LIMBS..].iter() {
        let t = builder.mul_extension(filter, x);
        yield_constr.constraint_transition(builder, t);
    }

    // constr_poly = c(x) + q(x) * m(x)
    let mut constr_poly: [_; 2 * N_LIMBS] = prod[0..2 * N_LIMBS].try_into().unwrap();
    pol_add_assign_ext_circuit(builder, &mut constr_poly, &output);

    let offset =
        builder.constant_extension(F::Extension::from_canonical_u64(AUX_COEFF_ABS_MAX as u64));
    let zero = builder.zero_extension();

    // constr_poly = c(x) + q(x) * m(x)
    let mut aux = [zero; 2 * N_LIMBS];
    for (c, i) in aux.iter_mut().zip(MODULAR_AUX_INPUT_LO) {
        *c = builder.sub_extension(nv[i], offset);
    }
    // add high 16-bits of aux input
    let base = F::from_canonical_u64(1u64 << LIMB_BITS);
    for (c, j) in aux.iter_mut().zip(MODULAR_AUX_INPUT_HI) {
        *c = builder.mul_const_add_extension(base, nv[j], *c);
    }

    let base = builder.constant_extension(base.into());
    let t = pol_adjoin_root_ext_circuit(builder, aux, base);
    pol_add_assign_ext_circuit(builder, &mut constr_poly, &t);

    constr_poly
}

pub(crate) fn submod_constr_poly_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    nv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    builder: &mut CircuitBuilder<F, D>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    filter: ExtensionTarget<D>,
    output: [ExtensionTarget<D>; N_LIMBS],
    modulus: [ExtensionTarget<D>; N_LIMBS],
    mut quot: [ExtensionTarget<D>; 2 * N_LIMBS],
) -> [ExtensionTarget<D>; 2 * N_LIMBS] {
    // quot was offset by 2^16 - 1 if it was negative; we undo that
    // offset here:
    let (lo, hi) = quot.split_at_mut(N_LIMBS);
    let sign = hi[0];
    let t = builder.mul_sub_extension(sign, sign, sign);
    let t = builder.mul_extension(filter, t);
    // sign must be 1 (negative) or 0 (positive)
    yield_constr.constraint(builder, t);
    let offset = F::from_canonical_u16(u16::max_value());
    for c in lo {
        let t = builder.mul_const_extension(offset, sign);
        *c = builder.sub_extension(*c, t);
    }
    hi[0] = builder.zero_extension();
    for d in hi {
        // All higher limbs must be zero
        let t = builder.mul_extension(filter, *d);
        yield_constr.constraint(builder, t);
    }

    modular_constr_poly_ext_circuit(lv, nv, builder, yield_constr, filter, output, modulus, quot)
}

pub(crate) fn eval_ext_circuit<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    lv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    nv: &[ExtensionTarget<D>; NUM_ARITH_COLUMNS],
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let bn254_filter = builder.add_many_extension([
        lv[columns::IS_ADDFP254],
        lv[columns::IS_MULFP254],
        lv[columns::IS_SUBFP254],
    ]);
    let filter = builder.add_many_extension([
        lv[columns::IS_ADDMOD],
        lv[columns::IS_SUBMOD],
        lv[columns::IS_MULMOD],
        bn254_filter,
    ]);

    // Ensure that this operation is not the last row of the table;
    // needed because we access the next row of the table in nv.
    yield_constr.constraint_last_row(builder, filter);

    // Verify that the modulus is the BN254 modulus for the
    // {ADD,MUL,SUB}FP254 operations.
    let modulus = read_value::<N_LIMBS, _>(lv, MODULAR_MODULUS);
    for (&mi, bi) in modulus.iter().zip(bn254_modulus_limbs()) {
        // bn254_filter * (mi - bi)
        let t = builder.arithmetic_extension(
            F::ONE,
            -F::from_canonical_u16(bi),
            mi,
            bn254_filter,
            bn254_filter,
        );
        yield_constr.constraint_transition(builder, t);
    }

    let output = read_value::<N_LIMBS, _>(lv, MODULAR_OUTPUT);
    let quo_input = read_value::<{ 2 * N_LIMBS }, _>(lv, MODULAR_QUO_INPUT);

    let add_filter = builder.add_extension(lv[columns::IS_ADDMOD], lv[columns::IS_ADDFP254]);
    let sub_filter = builder.add_extension(lv[columns::IS_SUBMOD], lv[columns::IS_SUBFP254]);
    let mul_filter = builder.add_extension(lv[columns::IS_MULMOD], lv[columns::IS_MULFP254]);
    let addmul_filter = builder.add_extension(add_filter, mul_filter);

    // constr_poly has 2*N_LIMBS limbs
    let submod_constr_poly = submod_constr_poly_ext_circuit(
        lv,
        nv,
        builder,
        yield_constr,
        sub_filter,
        output,
        modulus,
        quo_input,
    );
    let modular_constr_poly = modular_constr_poly_ext_circuit(
        lv,
        nv,
        builder,
        yield_constr,
        addmul_filter,
        output,
        modulus,
        quo_input,
    );
    let input0 = read_value(lv, MODULAR_INPUT_0);
    let input1 = read_value(lv, MODULAR_INPUT_1);

    let add_input = pol_add_ext_circuit(builder, input0, input1);
    let sub_input = pol_sub_ext_circuit(builder, input0, input1);
    let mul_input = pol_mul_wide_ext_circuit(builder, input0, input1);

    for (input, &filter, constr_poly) in [
        (&add_input, &add_filter, modular_constr_poly),
        (&sub_input, &sub_filter, submod_constr_poly),
        (&mul_input, &mul_filter, modular_constr_poly),
    ] {
        let mut constr_poly_copy = constr_poly;
        pol_sub_assign_ext_circuit(builder, &mut constr_poly_copy, input);
        for &c in constr_poly_copy.iter() {
            let t = builder.mul_extension(filter, c);
            yield_constr.constraint_transition(builder, t);
        }
    }
}

#[cfg(test)]
mod tests {
    use plonky2::field::goldilocks_field::GoldilocksField;
    use plonky2::field::types::{Field, Sample};
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    use super::*;
    use crate::arithmetic::columns::NUM_ARITH_COLUMNS;
    use crate::constraint_consumer::ConstraintConsumer;
    use crate::extension_tower::BN_BASE;

    const N_RND_TESTS: usize = 1000;
    const MODULAR_OPS: [usize; 6] = [
        IS_ADDMOD,
        IS_SUBMOD,
        IS_MULMOD,
        IS_ADDFP254,
        IS_SUBFP254,
        IS_MULFP254,
    ];

    // TODO: Should be able to refactor this test to apply to all operations.
    #[test]
    fn generate_eval_consistency_not_modular() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);
        let mut lv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::sample(&mut rng));
        let nv = [F::default(); NUM_ARITH_COLUMNS].map(|_| F::sample(&mut rng));

        // if `IS_ADDMOD == 0`, then the constraints should be met even
        // if all values are garbage (and similarly for the other operations).
        for op in MODULAR_OPS {
            lv[op] = F::ZERO;
        }
        lv[IS_SHR] = F::ZERO;
        lv[IS_DIV] = F::ZERO;
        lv[IS_MOD] = F::ZERO;

        let mut constraint_consumer = ConstraintConsumer::new(
            vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
            GoldilocksField::ONE,
            GoldilocksField::ONE,
            GoldilocksField::ONE,
        );
        eval_packed(&lv, &nv, &mut constraint_consumer);
        for &acc in &constraint_consumer.constraint_accs {
            assert_eq!(acc, GoldilocksField::ZERO);
        }
    }

    #[test]
    fn generate_eval_consistency() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);

        for op_filter in MODULAR_OPS {
            for i in 0..N_RND_TESTS {
                // set inputs to random values
                let mut lv = [F::default(); NUM_ARITH_COLUMNS]
                    .map(|_| F::from_canonical_u16(rng.gen::<u16>()));
                let mut nv = [F::default(); NUM_ARITH_COLUMNS]
                    .map(|_| F::from_canonical_u16(rng.gen::<u16>()));

                // Reset operation columns, then select one
                for op in MODULAR_OPS {
                    lv[op] = F::ZERO;
                }
                lv[IS_SHR] = F::ZERO;
                lv[IS_DIV] = F::ZERO;
                lv[IS_MOD] = F::ZERO;
                lv[op_filter] = F::ONE;

                let input0 = U256::from(rng.gen::<[u8; 32]>());
                let input1 = U256::from(rng.gen::<[u8; 32]>());

                let modulus = if [IS_ADDFP254, IS_MULFP254, IS_SUBFP254].contains(&op_filter) {
                    BN_BASE
                } else {
                    let mut modulus_limbs = [0u8; 32];
                    // For the second half of the tests, set the top
                    // 16-start digits of the modulus to zero so it is
                    // much smaller than the inputs.
                    if i > N_RND_TESTS / 2 {
                        // 1 <= start < N_LIMBS
                        let start = (rng.gen::<usize>() % (modulus_limbs.len() - 1)) + 1;
                        for mi in modulus_limbs.iter_mut().skip(start) {
                            *mi = 0u8;
                        }
                    }
                    U256::from(modulus_limbs)
                };

                generate(&mut lv, &mut nv, op_filter, input0, input1, modulus);

                let mut constraint_consumer = ConstraintConsumer::new(
                    vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
                    GoldilocksField::ONE,
                    GoldilocksField::ZERO,
                    GoldilocksField::ZERO,
                );
                eval_packed(&lv, &nv, &mut constraint_consumer);
                for &acc in &constraint_consumer.constraint_accs {
                    assert_eq!(acc, GoldilocksField::ZERO);
                }
            }
        }
    }

    #[test]
    fn zero_modulus() {
        type F = GoldilocksField;

        let mut rng = ChaCha8Rng::seed_from_u64(0x6feb51b7ec230f25);

        for op_filter in [IS_ADDMOD, IS_SUBMOD, IS_MULMOD] {
            for _i in 0..N_RND_TESTS {
                // set inputs to random values and the modulus to zero;
                // the output is defined to be zero when modulus is zero.
                let mut lv = [F::default(); NUM_ARITH_COLUMNS]
                    .map(|_| F::from_canonical_u16(rng.gen::<u16>()));
                let mut nv = [F::default(); NUM_ARITH_COLUMNS]
                    .map(|_| F::from_canonical_u16(rng.gen::<u16>()));

                // Reset operation columns, then select one
                for op in MODULAR_OPS {
                    lv[op] = F::ZERO;
                }
                lv[IS_SHR] = F::ZERO;
                lv[IS_DIV] = F::ZERO;
                lv[IS_MOD] = F::ZERO;
                lv[op_filter] = F::ONE;

                let input0 = U256::from(rng.gen::<[u8; 32]>());
                let input1 = U256::from(rng.gen::<[u8; 32]>());
                let modulus = U256::zero();

                generate(&mut lv, &mut nv, op_filter, input0, input1, modulus);

                // check that the correct output was generated
                assert!(lv[MODULAR_OUTPUT].iter().all(|&c| c == F::ZERO));

                let mut constraint_consumer = ConstraintConsumer::new(
                    vec![GoldilocksField(2), GoldilocksField(3), GoldilocksField(5)],
                    GoldilocksField::ONE,
                    GoldilocksField::ZERO,
                    GoldilocksField::ZERO,
                );
                eval_packed(&lv, &nv, &mut constraint_consumer);
                assert!(constraint_consumer
                    .constraint_accs
                    .iter()
                    .all(|&acc| acc == F::ZERO));

                // Corrupt one output limb by setting it to a non-zero value
                let random_oi = MODULAR_OUTPUT.start + rng.gen::<usize>() % N_LIMBS;
                lv[random_oi] = F::from_canonical_u16(rng.gen_range(1..u16::MAX));

                eval_packed(&lv, &nv, &mut constraint_consumer);

                // Check that at least one of the constraints was non-zero
                assert!(constraint_consumer
                    .constraint_accs
                    .iter()
                    .any(|&acc| acc != F::ZERO));
            }
        }
    }
}
