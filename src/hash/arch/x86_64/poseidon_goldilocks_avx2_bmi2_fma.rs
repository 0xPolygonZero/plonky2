use core::arch::x86_64::*;
use std::convert::TryInto;
use std::mem::size_of;

use static_assertions::const_assert;

use crate::field::field_types::Field;
use crate::field::goldilocks_field::GoldilocksField;
use crate::hash::poseidon::{
    Poseidon, ALL_ROUND_CONSTANTS, HALF_N_FULL_ROUNDS, N_PARTIAL_ROUNDS, N_ROUNDS,
};

// WARNING: This code contains tricks that work for the current MDS matrix and round constants, but
// are not guaranteed to work if those are changed.

// * Constant definitions *

const WIDTH: usize = 12;

// These tranformed round constants are used where the constant layer is fused with the preceeding
// MDS layer. The FUSED_ROUND_CONSTANTS for round i are the ALL_ROUND_CONSTANTS for round i + 1.
// The FUSED_ROUND_CONSTANTS for the very last round are 0, as it is not followed by a constant
// layer. On top of that, all FUSED_ROUND_CONSTANTS are shifted by 2 ** 63 to save a few XORs per
// round.
const fn make_fused_round_constants() -> [u64; WIDTH * N_ROUNDS] {
    let mut res = [0x8000000000000000u64; WIDTH * N_ROUNDS];
    let mut i: usize = WIDTH;
    while i < WIDTH * N_ROUNDS {
        res[i - WIDTH] ^= ALL_ROUND_CONSTANTS[i];
        i += 1;
    }
    res
}
const FUSED_ROUND_CONSTANTS: [u64; WIDTH * N_ROUNDS] = make_fused_round_constants();

// This is the top row of the MDS matrix. Concretely, it's the MDS exps vector at the following
// indices: [0, 11, ..., 1].
static TOP_ROW_EXPS: [usize; 12] = [0, 10, 16, 3, 12, 8, 1, 5, 3, 0, 1, 0];

// * Compile-time checks *

/// The MDS matrix multiplication ASM is specific to the MDS matrix below. We want this file to
/// fail to compile if it has been changed.
#[allow(dead_code)]
const fn check_mds_matrix() -> bool {
    // Can't == two arrays in a const_assert! (:
    let mut i = 0;
    let wanted_matrix_exps = [0, 0, 1, 0, 3, 5, 1, 8, 12, 3, 16, 10];
    while i < WIDTH {
        if <GoldilocksField as Poseidon<12>>::MDS_MATRIX_EXPS[i] != wanted_matrix_exps[i] {
            return false;
        }
        i += 1;
    }
    true
}
const_assert!(check_mds_matrix());

/// The maximum amount by which the MDS matrix will multiply the input.
/// i.e. max(MDS(state)) <= mds_matrix_inf_norm() * max(state).
const fn mds_matrix_inf_norm() -> u64 {
    let mut cumul = 0;
    let mut i = 0;
    while i < WIDTH {
        cumul += 1 << <GoldilocksField as Poseidon<12>>::MDS_MATRIX_EXPS[i];
        i += 1;
    }
    cumul
}

/// Ensure that adding round constants to the low result of the MDS multiplication can never
/// overflow.
#[allow(dead_code)]
const fn check_round_const_bounds_mds() -> bool {
    let max_mds_res = mds_matrix_inf_norm() * (u32::MAX as u64);
    let mut i = WIDTH; // First const layer is handled specially.
    while i < WIDTH * N_ROUNDS {
        if ALL_ROUND_CONSTANTS[i].overflowing_add(max_mds_res).1 {
            return false;
        }
        i += 1;
    }
    true
}
const_assert!(check_round_const_bounds_mds());

/// Ensure that the first WIDTH round constants are in canonical form for the vpcmpgtd trick.
#[allow(dead_code)]
const fn check_round_const_bounds_init() -> bool {
    let max_permitted_round_const = 0xffffffff00000000;
    let mut i = 0; // First const layer is handled specially.
    while i < WIDTH {
        if ALL_ROUND_CONSTANTS[i] > max_permitted_round_const {
            return false;
        }
        i += 1;
    }
    true
}
const_assert!(check_round_const_bounds_init());

// Preliminary notes:
// 1. AVX does not support addition with carry but 128-bit (2-word) addition can be easily
//    emulated. The method recognizes that for a + b overflowed iff (a + b) < a:
//        i. res_lo = a_lo + b_lo
//       ii. carry_mask = res_lo < a_lo
//      iii. res_hi = a_hi + b_hi - carry_mask
//    Notice that carry_mask is subtracted, not added. This is because AVX comparison instructions
//    return -1 (all bits 1) for true and 0 for false.
//
// 2. AVX does not have unsigned 64-bit comparisons. Those can be emulated with signed comparisons
//    by recognizing that a <u b iff a + (1 << 63) <s b + (1 << 63), where the addition wraps around
//    and the comparisons are unsigned and signed respectively. The shift function adds/subtracts
//    1 << 63 to enable this trick.
//      Example: addition with carry.
//        i. a_lo_s = shift(a_lo)
//       ii. res_lo_s = a_lo_s + b_lo
//      iii. carry_mask = res_lo_s <s a_lo_s
//       iv. res_lo = shift(res_lo_s)
//        v. res_hi = a_hi + b_hi - carry_mask
//    The suffix _s denotes a value that has been shifted by 1 << 63. The result of addition is
//    shifted if exactly one of the operands is shifted, as is the case on line ii. Line iii.
//    performs a signed comparison res_lo_s <s a_lo_s on shifted values to emulate unsigned
//    comparison res_lo <u a_lo on unshifted values. Finally, line iv. reverses the shift so the
//    result can be returned.
//      When performing a chain of calculations, we can often save instructions by letting the shift
//    propagate through and only undoing it when necessary. For example, to compute the addition of
//    three two-word (128-bit) numbers we can do:
//        i. a_lo_s = shift(a_lo)
//       ii. tmp_lo_s = a_lo_s + b_lo
//      iii. tmp_carry_mask = tmp_lo_s <s a_lo_s
//       iv. tmp_hi = a_hi + b_hi - tmp_carry_mask
//        v. res_lo_s = tmp_lo_s + c_lo
//       vi. res_carry_mask = res_lo_s <s tmp_lo_s
//      vii. res_lo = shift(res_lo_s)
//     viii. res_hi = tmp_hi + c_hi - res_carry_mask
//    Notice that the above 3-value addition still only requires two calls to shift, just like our
//    2-value addition.

macro_rules! map3 {
    ($f:ident::<$l:literal>, $v:ident) => {
        ($f::<$l>($v.0), $f::<$l>($v.1), $f::<$l>($v.2))
    };
    ($f:ident, $v0:ident, $v1:ident) => {
        ($f($v0.0, $v1.0), $f($v0.1, $v1.1), $f($v0.2, $v1.2))
    };
    ($f:ident, $v0:ident, rep $v1:ident) => {
        ($f($v0.0, $v1), $f($v0.1, $v1), $f($v0.2, $v1))
    };
}

#[inline(always)]
unsafe fn const_layer(
    state: (__m256i, __m256i, __m256i),
    round_const_arr: &[u64; 12],
) -> (__m256i, __m256i, __m256i) {
    let sign_bit = _mm256_set1_epi64x(i64::MIN);
    let round_const = (
        _mm256_loadu_si256((&round_const_arr[0..4]).as_ptr().cast::<__m256i>()),
        _mm256_loadu_si256((&round_const_arr[4..8]).as_ptr().cast::<__m256i>()),
        _mm256_loadu_si256((&round_const_arr[8..12]).as_ptr().cast::<__m256i>()),
    );
    let state_s = map3!(_mm256_xor_si256, state, rep sign_bit); // Shift by 2**63.
    let res_maybe_wrapped_s = map3!(_mm256_add_epi64, state_s, round_const);
    // 32-bit compare is much faster than 64-bit compare on Intel. We can use 32-bit compare here
    // as long as we can guarantee that state > res_maybe_wrapped iff state >> 32 >
    // res_maybe_wrapped >> 32. Clearly, if state >> 32 > res_maybe_wrapped >> 32, then state >
    // res_maybe_wrapped, and similarly for <.
    //   It remains to show that we can't have state >> 32 == res_maybe_wrapped >> 32 with state >
    // res_maybe_wrapped. If state >> 32 == res_maybe_wrapped >> 32, then round_const >> 32 =
    // 0xffffffff and the addition of the low doubleword generated a carry bit. This can never
    // occur if all round constants are < 0xffffffff00000001 = ORDER: if the high bits are
    // 0xffffffff, then the low bits are 0, so the carry bit cannot occur. So this trick is valid
    // as long as all the round constants are in canonical form.
    // The mask contains 0xffffffff in the high doubleword if wraparound occured and 0 otherwise.
    // We will ignore the low doubleword.
    let wraparound_mask = map3!(_mm256_cmpgt_epi32, state_s, res_maybe_wrapped_s);
    // wraparound_adjustment contains 0xffffffff = EPSILON if wraparound occured and 0 otherwise.
    let wraparound_adjustment = map3!(_mm256_srli_epi64::<32>, wraparound_mask);
    // XOR commutes with the addition below. Placing it here helps mask latency.
    let res_maybe_wrapped = map3!(_mm256_xor_si256, res_maybe_wrapped_s, rep sign_bit);
    // Add EPSILON = subtract ORDER.
    let res = map3!(_mm256_add_epi64, res_maybe_wrapped, wraparound_adjustment);
    res
}

#[inline(always)]
unsafe fn square3(
    x: (__m256i, __m256i, __m256i),
) -> ((__m256i, __m256i, __m256i), (__m256i, __m256i, __m256i)) {
    let sign_bit = _mm256_set1_epi64x(i64::MIN);
    let x_hi = map3!(_mm256_srli_epi64::<32>, x);
    let mul_ll = map3!(_mm256_mul_epu32, x, x);
    let mul_lh = map3!(_mm256_mul_epu32, x, x_hi);
    let mul_hh = map3!(_mm256_mul_epu32, x_hi, x_hi);
    let res_lo0_s = map3!(_mm256_xor_si256, mul_ll, rep sign_bit);
    let mul_lh_lo = map3!(_mm256_slli_epi64::<33>, mul_lh);
    let res_lo1_s = map3!(_mm256_add_epi64, res_lo0_s, mul_lh_lo);
    let carry = map3!(_mm256_cmpgt_epi64, res_lo0_s, res_lo1_s);
    let mul_lh_hi = map3!(_mm256_srli_epi64::<31>, mul_lh);
    let res_hi0 = map3!(_mm256_add_epi64, mul_hh, mul_lh_hi);
    let res_hi1 = map3!(_mm256_sub_epi64, res_hi0, carry);
    (res_lo1_s, res_hi1)
}

#[inline(always)]
unsafe fn mul3(
    x: (__m256i, __m256i, __m256i),
    y: (__m256i, __m256i, __m256i),
) -> ((__m256i, __m256i, __m256i), (__m256i, __m256i, __m256i)) {
    let sign_bit = _mm256_set1_epi64x(i64::MIN);
    let y_hi = map3!(_mm256_srli_epi64::<32>, y);
    let x_hi = map3!(_mm256_srli_epi64::<32>, x);
    let mul_ll = map3!(_mm256_mul_epu32, x, y);
    let mul_lh = map3!(_mm256_mul_epu32, x, y_hi);
    let mul_hl = map3!(_mm256_mul_epu32, x_hi, y);
    let mul_hh = map3!(_mm256_mul_epu32, x_hi, y_hi);
    let mul_lh_lo = map3!(_mm256_slli_epi64::<32>, mul_lh);
    let res_lo0_s = map3!(_mm256_xor_si256, mul_ll, rep sign_bit);
    let mul_hl_lo = map3!(_mm256_slli_epi64::<32>, mul_hl);
    let res_lo1_s = map3!(_mm256_add_epi64, res_lo0_s, mul_lh_lo);
    let carry0 = map3!(_mm256_cmpgt_epi64, res_lo0_s, res_lo1_s);
    let mul_lh_hi = map3!(_mm256_srli_epi64::<32>, mul_lh);
    let res_lo2_s = map3!(_mm256_add_epi64, res_lo1_s, mul_hl_lo);
    let carry1 = map3!(_mm256_cmpgt_epi64, res_lo1_s, res_lo2_s);
    let mul_hl_hi = map3!(_mm256_srli_epi64::<32>, mul_hl);
    let res_hi0 = map3!(_mm256_add_epi64, mul_hh, mul_lh_hi);
    let res_hi1 = map3!(_mm256_add_epi64, res_hi0, mul_hl_hi);
    let res_hi2 = map3!(_mm256_sub_epi64, res_hi1, carry0);
    let res_hi3 = map3!(_mm256_sub_epi64, res_hi2, carry1);
    (res_lo2_s, res_hi3)
}

#[inline(always)]
unsafe fn reduce3(
    (x_lo_s, x_hi): ((__m256i, __m256i, __m256i), (__m256i, __m256i, __m256i)),
) -> (__m256i, __m256i, __m256i) {
    let epsilon = _mm256_set1_epi64x(0xffffffff);
    let sign_bit = _mm256_set1_epi64x(i64::MIN);
    let x_hi_hi = map3!(_mm256_srli_epi64::<32>, x_hi);
    let res0_s = map3!(_mm256_sub_epi64, x_lo_s, x_hi_hi);
    let wraparound_mask0 = map3!(_mm256_cmpgt_epi32, res0_s, x_lo_s);
    let wraparound_adj0 = map3!(_mm256_srli_epi64::<32>, wraparound_mask0);
    let x_hi_lo = map3!(_mm256_and_si256, x_hi, rep epsilon);
    let x_hi_lo_shifted = map3!(_mm256_slli_epi64::<32>, x_hi);
    let res1_s = map3!(_mm256_sub_epi64, res0_s, wraparound_adj0);
    let x_hi_lo_mul_epsilon = map3!(_mm256_sub_epi64, x_hi_lo_shifted, x_hi_lo);
    let res2_s = map3!(_mm256_add_epi64, res1_s, x_hi_lo_mul_epsilon);
    let wraparound_mask2 = map3!(_mm256_cmpgt_epi32, res1_s, res2_s);
    let wraparound_adj2 = map3!(_mm256_srli_epi64::<32>, wraparound_mask2);
    let res3_s = map3!(_mm256_add_epi64, res2_s, wraparound_adj2);
    let res3 = map3!(_mm256_xor_si256, res3_s, rep sign_bit);
    res3
}

#[inline(always)]
unsafe fn sbox_layer_full(state: (__m256i, __m256i, __m256i)) -> (__m256i, __m256i, __m256i) {
    let state2_unreduced = square3(state);
    let state2 = reduce3(state2_unreduced);
    let state4_unreduced = square3(state2);
    let state3_unreduced = mul3(state2, state);
    let state4 = reduce3(state4_unreduced);
    let state3 = reduce3(state3_unreduced);
    let state7_unreduced = mul3(state3, state4);
    let state7 = reduce3(state7_unreduced);
    state7
}

#[inline(always)]
unsafe fn mds_layer_reduce(
    lo_s: (__m256i, __m256i, __m256i),
    hi: (__m256i, __m256i, __m256i),
) -> (__m256i, __m256i, __m256i) {
    // This is done in assembly because, frankly, it's cleaner than intrinsics. We also don't have
    // to worry about whether the compiler is doing weird things. This entire routine needs proper
    // pipelining so there's no point rewriting this, only to have to rewrite it again.
    let res0: __m256i;
    let res1: __m256i;
    let res2: __m256i;
    let epsilon = _mm256_set1_epi64x(0xffffffff);
    let sign_bit = _mm256_set1_epi64x(i64::MIN);
    asm!(
        // The high results are in ymm3, ymm4, ymm5.
        // The low results (shifted by 2**63) are in ymm0, ymm1, ymm2

        // We want to do: ymm0 := ymm0 + (ymm3 * 2**32) in modulo P.
        // This can be computed by ymm0 + (ymm3 << 32) + (ymm3 >> 32) * EPSILON,
        // where the additions must correct for over/underflow.

        // First, do ymm0 + (ymm3 << 32)  (first chain)
        "vpsllq   ymm6, ymm3, 32",
        "vpsllq   ymm7, ymm4, 32",
        "vpsllq   ymm8, ymm5, 32",
        "vpaddq   ymm6, ymm6, ymm0",
        "vpaddq   ymm7, ymm7, ymm1",
        "vpaddq   ymm8, ymm8, ymm2",
        "vpcmpgtd ymm0, ymm0, ymm6",
        "vpcmpgtd ymm1, ymm1, ymm7",
        "vpcmpgtd ymm2, ymm2, ymm8",

        // Now we interleave the chains so this gets a bit uglier.
        // Form ymm3 := (ymm3 >> 32) * EPSILON  (second chain)
        "vpsrlq ymm9,  ymm3, 32",
        "vpsrlq ymm10, ymm4, 32",
        "vpsrlq ymm11, ymm5, 32",
        // (first chain again)
        "vpsrlq ymm0, ymm0, 32",
        "vpsrlq ymm1, ymm1, 32",
        "vpsrlq ymm2, ymm2, 32",
        // (second chain again)
        "vpandn ymm3, ymm14, ymm3",
        "vpandn ymm4, ymm14, ymm4",
        "vpandn ymm5, ymm14, ymm5",
        "vpsubq ymm3, ymm3, ymm9",
        "vpsubq ymm4, ymm4, ymm10",
        "vpsubq ymm5, ymm5, ymm11",
        // (first chain again)
        "vpaddq ymm0, ymm6, ymm0",
        "vpaddq ymm1, ymm7, ymm1",
        "vpaddq ymm2, ymm8, ymm2",

        // Merge two chains (second addition)
        "vpaddq   ymm3, ymm0, ymm3",
        "vpaddq   ymm4, ymm1, ymm4",
        "vpaddq   ymm5, ymm2, ymm5",
        "vpcmpgtd ymm0, ymm0, ymm3",
        "vpcmpgtd ymm1, ymm1, ymm4",
        "vpcmpgtd ymm2, ymm2, ymm5",
        "vpsrlq   ymm6, ymm0, 32",
        "vpsrlq   ymm7, ymm1, 32",
        "vpsrlq   ymm8, ymm2, 32",
        "vpxor    ymm3, ymm15, ymm3",
        "vpxor    ymm4, ymm15, ymm4",
        "vpxor    ymm5, ymm15, ymm5",
        "vpaddq   ymm0, ymm6, ymm3",
        "vpaddq   ymm1, ymm7, ymm4",
        "vpaddq   ymm2, ymm8, ymm5",
        inout("ymm0") lo_s.0 => res0,
        inout("ymm1") lo_s.1 => res1,
        inout("ymm2") lo_s.2 => res2,
        inout("ymm3") hi.0 => _,
        inout("ymm4") hi.1 => _,
        inout("ymm5") hi.2 => _,
        out("ymm6") _, out("ymm7") _, out("ymm8") _, out("ymm9") _, out("ymm10") _, out("ymm11") _,
        in("ymm14") epsilon, in("ymm15") sign_bit,
        options(pure, nomem, preserves_flags, nostack),
    );
    (res0, res1, res2)
}

/// Given a vector of quadwords, represent it as two vectors of double-precision floats, usable in
/// MDS multiplication (as input to fused multiply-add).
///
/// More precisely, let x = 0xKLMNOPQRSTUVWXYZ. We return lo = 0x4230STUVWXYZ0000 and
/// hi = 0x4230KLMNOPQR0000. They encode the integer 0x10STUVWXYZ (resp. 0x10KLMNOPQR), in
/// floating-point: sign (1 bit) of 0, exponent (11 bits) of 0x423, and a significand (52 bits) of
/// 0x0STUVWXYZ0000 (resp. 0x0KLMNOPQR0000).
///
/// Note therefore that this function does not strictly convert each doubleword to a
/// double-precision float. The resulting float is the lo/hi doubleword plus 0x1000000000.
#[inline(always)]
unsafe fn convert_to_floatish(x: __m256i) -> (__m256d, __m256d) {
    // The exponent of 0x423 is fairly arbitrary. The resulting float is an encoding of
    // 0x10STUVWXYZ * 2 ** n, where n depends on the exponent, and 0x423 sets n = 0. But other
    // values, except 0 (which is denormal) and really large values (which will cause overflow
    // later on), would also work.

    // The shift by 0x1000000000 makes our life easier in two ways. Firstly, we don't have to worry
    // about finding the exponent when converting to floating point: it's a constant value.
    // Secondly the exponent of the MDS multiplication result is also guaranteed to be constant, so
    // we don't have to decode it. By linearity of matrix multiplication, the appropriate multiple
    // of 0x1000000000 can simply be subtracted from the result.

    // The gap of 4 zeros at the start of the significand is not arbitrary. It ensures that the
    // least significant bit of the MDS multiplication result (decoded as a uint64) corresponds to
    // 1 (in floating-point). In other words, we can convert from floating-point to integer just by
    // subtracting the floating-point exponent (a constant) and a multiple of 0x1000000000 (another
    // constant).

    let consts = _mm256_set1_epi32(0x00004230); // consts == 0x0000423000004230
    // lo doubleword from x, hi doubleword from consts: lo_unshifted == 0x00004230STUVWXYZ
    let lo_unshifted = _mm256_blend_epi32::<0xaa>(x, consts);
    // lo doubleword from const, hi doubleword from x: hi_unshifted == 0xKLMNOPQR00004230
    let hi_unshifted = _mm256_blend_epi32::<0x55>(x, consts);
    // Shift quadword left by 16: lo == 0x4230STUVWXYZ0000
    let lo = _mm256_slli_epi64::<16>(lo_unshifted);
    // Hack: rotate each 16-byte block (2 quadwords) right by 2 bytes (16 bits).
    let hi = _mm256_alignr_epi8::<2>(hi_unshifted, hi_unshifted); // hi == 0x4230KLMNOPQR0000
    (_mm256_castsi256_pd(lo), _mm256_castsi256_pd(hi)) // Bit-casts are a no-op in ASM
}

/// Convert lo MDS multiplication result from floating-point, add round constants, and shift by
/// 2 ** 63.
///
/// Written to complement convert_to_floatish. This routine cancels out the shift by 0x1000000000
/// added there.
///
/// post_mds_round_constants is a vector of round constants transformed in a very particular way:
/// the round constant C_ri must be a in canonical form. post_mds_round_constants is set to
/// C_ri + 0x3cceac9000000000 (with wraparound).
#[inline(always)]
unsafe fn convert_from_floatish_lo_and_add_round_constants_s(
    x: __m256d,
    post_mds_round_constants: [u64; 4],
) -> __mm256i {
    // Explanation of the magic constant: 0x3cceac9000000000 = -E - M + S, where E =
    // 0x4330000000000000 subtracts the exponent (leaving only the significand), M =
    // 0x0001537000000000 cancels the shift by 0x1000000000 (note that M = 0x1000000000 *
    // sum(1 << i for i in MDS_EXPS)), and S = 0x8000000000000000 shifts by 2**63.
    let round_consts = _mm256_loadu_si256(post_mds_round_constants.as_ptr().cast::<__m256i>());
    let xi = _mm256_castpd_si256(x); // Bit-cast (free).
    _mm256_add_epi64(xi, round_consts)
}

/// Convert hi MDS multiplication result from floating-point.
///
/// Written to complement convert_to_floatish. This routine cancels out the shift by 0x1000000000
/// added there.
#[inline(always)]
unsafe fn convert_from_floatish_hi(x: __m256d) -> __m256i {
    // Sum of:
    //   1. 0x4330000000000000, which subtracts the exponent, and
    //   2. 0x0001537000000000 = 0x1000000000 * sum(1 << i for i in MDS_EXPS), which cancels the
    //      shift by 0x1000000000.
    let magic_const = _mm256_set1_epi64x(0x4331537000000000);
    let xi = _mm256_castpd_si256(x); // Bit-cast (free).
    _mm256_sub_epi64(xi, magic_const)
}

#[inline(always)]
unsafe fn mds_multiply_and_add_round_const_s(
    state: (__m256i, __m256i, __m256i),
    (base, index): (*const u64, usize),
) -> ((__m256i, __m256i, __m256i), (__m256i, __m256i, __m256i)) {

}

#[inline(always)]
unsafe fn mds_const_layers_full(
    state: (__m256i, __m256i, __m256i),
    round_constants: (*const u64, usize),
) -> (__m256i, __m256i, __m256i) {
    let (unreduced_lo_s, unreduced_hi) = mds_multiply_and_add_round_const_s(state, round_constants);
    mds_layer_reduce(unreduced_lo_s, unreduced_hi)
}

/// Compute x ** 7
#[inline(always)]
unsafe fn sbox_partial(mut x: u64) -> u64 {
    // This is done in assembly to fix LLVM's poor treatment of wraparound addition/subtraction
    // and to ensure that multiplication by EPSILON is done with bitshifts, leaving port 1 for
    // vector operations.
    // TODO: Interleave with MDS multiplication.
    asm!(
        "mov r9, rdx",

        // rdx := rdx ^ 2
        "mulx rdx, rax, rdx",
        "shrx r8, rdx, r15",
        "mov r12d, edx",
        "shl rdx, 32",
        "sub rdx, r12",
        // rax - r8, with underflow
        "sub rax, r8",
        "sbb r8d, r8d", // sets r8 to 2^32 - 1 if subtraction underflowed
        "sub rax, r8",
        // rdx + rax, with overflow
        "add rdx, rax",
        "sbb eax, eax",
        "add rdx, rax",

        // rax := rdx * r9, rdx := rdx ** 2
        "mulx rax, r11, r9",
        "mulx rdx, r12, rdx",

        "shrx r9, rax, r15",
        "shrx r10, rdx, r15",

        "sub r11, r9",
        "sbb r9d, r9d",
        "sub r12, r10",
        "sbb r10d, r10d",
        "sub r11, r9",
        "sub r12, r10",

        "mov r9d, eax",
        "mov r10d, edx",
        "shl rax, 32",
        "shl rdx, 32",
        "sub rax, r9",
        "sub rdx, r10",

        "add rax, r11",
        "sbb r11d, r11d",
        "add rdx, r12",
        "sbb r12d, r12d",
        "add rax, r11",
        "add rdx, r12",

        // rax := rax * rdx
        "mulx rax, rdx, rax",
        "shrx r11, rax, r15",
        "mov r12d, eax",
        "shl rax, 32",
        "sub rax, r12",
        // rdx - r11, with underflow
        "sub rdx, r11",
        "sbb r11d, r11d", // sets r11 to 2^32 - 1 if subtraction underflowed
        "sub rdx, r11",
        //  rdx + rax, with overflow
        "add rdx, rax",
        "sbb eax, eax",
        "add rdx, rax",
        inout("rdx") x,
        out("rax") _,
        out("r8") _,
        out("r9") _,
        out("r10") _,
        out("r11") _,
        out("r12") _,
        in("r15") 32,
        options(pure, nomem, nostack),
    );
    x
}

#[inline(always)]
unsafe fn partial_round(
    (state0, state1, state2): (__m256i, __m256i, __m256i),
    round_constants: (*const u64, usize),
) -> (__m256i, __m256i, __m256i) {
    // Extract the low quadword
    let state0ab: __m128i = _mm256_castsi256_si128(state0);
    let mut state0a = _mm_cvtsi128_si64(state0ab) as u64;

    // Zero the low quadword
    let zero = _mm256_setzero_si256();
    let state0bcd = _mm256_blend_epi32::<0x3>(state0, zero);

    // Scalar exponentiation
    state0a = sbox_partial(state0a);

    let epsilon = _mm256_set1_epi64x(0xffffffff);
    let (
        (mut unreduced_lo0_s, mut unreduced_lo1_s, mut unreduced_lo2_s),
        (mut unreduced_hi0, mut unreduced_hi1, mut unreduced_hi2),
    ) = mds_multiply_and_add_round_const_s((state0bcd, state1, state2), round_constants);
    asm!(
        // Just done the MDS matrix multiplication on high 32 bits.
        // The high results are in ymm3, ymm4, ymm5.
        // The low results (shifted by 2**63) are in ymm0, ymm1, ymm2

        // The MDS matrix multiplication was done with state[0] set to 0.
        // We must:
        //  1. propagate the vector product to state[0], which is stored in rdx.
        //  2. offset state[1..12] by the appropriate multiple of rdx
        //  3. zero the lowest quadword in the vector registers
        "vmovq xmm12, {state0a}",
        "vpbroadcastq ymm12, xmm12",
        "vpsrlq ymm13, ymm12, 32",
        "vpand ymm12, ymm14, ymm12",

        // The current matrix-vector product goes not include state[0] as an input. (Imagine Mv
        // multiplication where we've set the first element to 0.) Add the remaining bits now.
        // TODO: This is a bit of an afterthought, which is why these constants are loaded 22
        //   times... There's likely a better way of merging those results.
        "vmovdqu ymm6, {mds_matrix}[rip]",
        "vmovdqu ymm7, {mds_matrix}[rip + 32]",
        "vmovdqu ymm8, {mds_matrix}[rip + 64]",
        "vpsllvq ymm9, ymm13, ymm6",
        "vpsllvq ymm10, ymm13, ymm7",
        "vpsllvq ymm11, ymm13, ymm8",
        "vpsllvq ymm6, ymm12, ymm6",
        "vpsllvq ymm7, ymm12, ymm7",
        "vpsllvq ymm8, ymm12, ymm8",
        "vpaddq  ymm3, ymm9, ymm3",
        "vpaddq  ymm4, ymm10, ymm4",
        "vpaddq  ymm5, ymm11, ymm5",
        "vpaddq  ymm0, ymm6, ymm0",
        "vpaddq  ymm1, ymm7, ymm1",
        "vpaddq  ymm2, ymm8, ymm2",
        // Reduction required.

        state0a = in(reg) state0a,
        mds_matrix = sym TOP_ROW_EXPS,
        inout("ymm0") unreduced_lo0_s,
        inout("ymm1") unreduced_lo1_s,
        inout("ymm2") unreduced_lo2_s,
        inout("ymm3") unreduced_hi0,
        inout("ymm4") unreduced_hi1,
        inout("ymm5") unreduced_hi2,
        out("ymm6") _,out("ymm7") _, out("ymm8") _, out("ymm9") _,
        out("ymm10") _, out("ymm11") _, out("ymm12") _, out("ymm13") _,
        in("ymm14") epsilon,
        options(pure, nomem, preserves_flags, nostack),
    );
    mds_layer_reduce(
        (unreduced_lo0_s, unreduced_lo1_s, unreduced_lo2_s),
        (unreduced_hi0, unreduced_hi1, unreduced_hi2),
    )
}

#[inline(always)]
unsafe fn full_round(
    state: (__m256i, __m256i, __m256i),
    round_constants: (*const u64, usize),
) -> (__m256i, __m256i, __m256i) {
    let state = sbox_layer_full(state);
    let state = mds_const_layers_full(state, round_constants);
    state
}

#[inline] // Called twice; permit inlining but don't _require_ it
unsafe fn half_full_rounds(
    mut state: (__m256i, __m256i, __m256i),
    start_round: usize,
) -> (__m256i, __m256i, __m256i) {
    let base = (&FUSED_ROUND_CONSTANTS
        [WIDTH * start_round..WIDTH * start_round + WIDTH * HALF_N_FULL_ROUNDS])
        .as_ptr();

    for i in 0..HALF_N_FULL_ROUNDS {
        state = full_round(state, (base, i * WIDTH * size_of::<u64>()));
    }
    state
}

#[inline(always)]
unsafe fn all_partial_rounds(
    mut state: (__m256i, __m256i, __m256i),
    start_round: usize,
) -> (__m256i, __m256i, __m256i) {
    let base = (&FUSED_ROUND_CONSTANTS
        [WIDTH * start_round..WIDTH * start_round + WIDTH * N_PARTIAL_ROUNDS])
        .as_ptr();

    for i in 0..N_PARTIAL_ROUNDS {
        state = partial_round(state, (base, i * WIDTH * size_of::<u64>()));
    }
    state
}

#[inline]
pub unsafe fn poseidon(state: &[GoldilocksField; 12]) -> [GoldilocksField; 12] {
    let state = (
        _mm256_loadu_si256((&state[0..4]).as_ptr().cast::<__m256i>()),
        _mm256_loadu_si256((&state[4..8]).as_ptr().cast::<__m256i>()),
        _mm256_loadu_si256((&state[8..12]).as_ptr().cast::<__m256i>()),
    );

    // The first constant layer must be done explicitly. The remaining constant layers are fused
    // with the preceeding MDS layer.
    let state = const_layer(state, &ALL_ROUND_CONSTANTS[0..WIDTH].try_into().unwrap());

    let state = half_full_rounds(state, 0);
    let state = all_partial_rounds(state, HALF_N_FULL_ROUNDS);
    let state = half_full_rounds(state, HALF_N_FULL_ROUNDS + N_PARTIAL_ROUNDS);

    let mut res = [GoldilocksField::ZERO; 12];
    _mm256_storeu_si256((&mut res[0..4]).as_mut_ptr().cast::<__m256i>(), state.0);
    _mm256_storeu_si256((&mut res[4..8]).as_mut_ptr().cast::<__m256i>(), state.1);
    _mm256_storeu_si256((&mut res[8..12]).as_mut_ptr().cast::<__m256i>(), state.2);

    res
}
