#![allow(clippy::assertions_on_constants)]

use core::arch::asm;
use core::arch::x86_64::*;
use core::mem::size_of;

use static_assertions::const_assert;

use crate::field::goldilocks_field::GoldilocksField;
use crate::field::types::Field;
use crate::hash::poseidon::{
    Poseidon, ALL_ROUND_CONSTANTS, HALF_N_FULL_ROUNDS, N_PARTIAL_ROUNDS, N_ROUNDS,
};
use crate::util::branch_hint;

// WARNING: This code contains tricks that work for the current MDS matrix and round constants, but
// are not guaranteed to work if those are changed.

// * Constant definitions *

const WIDTH: usize = 12;

// These transformed round constants are used where the constant layer is fused with the preceding
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
        if <GoldilocksField as Poseidon>::MDS_MATRIX_EXPS[i] != wanted_matrix_exps[i] {
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
        cumul += 1 << <GoldilocksField as Poseidon>::MDS_MATRIX_EXPS[i];
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
    ($f:ident::<$l:literal>, $v1:ident, $v2:ident) => {
        (
            $f::<$l>($v1.0, $v2.0),
            $f::<$l>($v1.1, $v2.1),
            $f::<$l>($v1.2, $v2.2),
        )
    };
    ($f:ident, $v:ident) => {
        ($f($v.0), $f($v.1), $f($v.2))
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
    // The mask contains 0xffffffff in the high doubleword if wraparound occurred and 0 otherwise.
    // We will ignore the low doubleword.
    let wraparound_mask = map3!(_mm256_cmpgt_epi32, state_s, res_maybe_wrapped_s);
    // wraparound_adjustment contains 0xffffffff = EPSILON if wraparound occurred and 0 otherwise.
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
    let x_hi = {
        // Move high bits to low position. The high bits of x_hi are ignored. Swizzle is faster than
        // bitshift. This instruction only has a floating-point flavor, so we cast to/from float.
        // This is safe and free.
        let x_ps = map3!(_mm256_castsi256_ps, x);
        let x_hi_ps = map3!(_mm256_movehdup_ps, x_ps);
        map3!(_mm256_castps_si256, x_hi_ps)
    };

    // All pairwise multiplications.
    let mul_ll = map3!(_mm256_mul_epu32, x, x);
    let mul_lh = map3!(_mm256_mul_epu32, x, x_hi);
    let mul_hh = map3!(_mm256_mul_epu32, x_hi, x_hi);

    // Bignum addition, but mul_lh is shifted by 33 bits (not 32).
    let mul_ll_hi = map3!(_mm256_srli_epi64::<33>, mul_ll);
    let t0 = map3!(_mm256_add_epi64, mul_lh, mul_ll_hi);
    let t0_hi = map3!(_mm256_srli_epi64::<31>, t0);
    let res_hi = map3!(_mm256_add_epi64, mul_hh, t0_hi);

    // Form low result by adding the mul_ll and the low 31 bits of mul_lh (shifted to the high
    // position).
    let mul_lh_lo = map3!(_mm256_slli_epi64::<33>, mul_lh);
    let res_lo = map3!(_mm256_add_epi64, mul_ll, mul_lh_lo);

    (res_lo, res_hi)
}

#[inline(always)]
unsafe fn mul3(
    x: (__m256i, __m256i, __m256i),
    y: (__m256i, __m256i, __m256i),
) -> ((__m256i, __m256i, __m256i), (__m256i, __m256i, __m256i)) {
    let epsilon = _mm256_set1_epi64x(0xffffffff);
    let x_hi = {
        // Move high bits to low position. The high bits of x_hi are ignored. Swizzle is faster than
        // bitshift. This instruction only has a floating-point flavor, so we cast to/from float.
        // This is safe and free.
        let x_ps = map3!(_mm256_castsi256_ps, x);
        let x_hi_ps = map3!(_mm256_movehdup_ps, x_ps);
        map3!(_mm256_castps_si256, x_hi_ps)
    };
    let y_hi = {
        let y_ps = map3!(_mm256_castsi256_ps, y);
        let y_hi_ps = map3!(_mm256_movehdup_ps, y_ps);
        map3!(_mm256_castps_si256, y_hi_ps)
    };

    // All four pairwise multiplications
    let mul_ll = map3!(_mm256_mul_epu32, x, y);
    let mul_lh = map3!(_mm256_mul_epu32, x, y_hi);
    let mul_hl = map3!(_mm256_mul_epu32, x_hi, y);
    let mul_hh = map3!(_mm256_mul_epu32, x_hi, y_hi);

    // Bignum addition
    // Extract high 32 bits of mul_ll and add to mul_hl. This cannot overflow.
    let mul_ll_hi = map3!(_mm256_srli_epi64::<32>, mul_ll);
    let t0 = map3!(_mm256_add_epi64, mul_hl, mul_ll_hi);
    // Extract low 32 bits of t0 and add to mul_lh. Again, this cannot overflow.
    // Also, extract high 32 bits of t0 and add to mul_hh.
    let t0_lo = map3!(_mm256_and_si256, t0, rep epsilon);
    let t0_hi = map3!(_mm256_srli_epi64::<32>, t0);
    let t1 = map3!(_mm256_add_epi64, mul_lh, t0_lo);
    let t2 = map3!(_mm256_add_epi64, mul_hh, t0_hi);
    // Lastly, extract the high 32 bits of t1 and add to t2.
    let t1_hi = map3!(_mm256_srli_epi64::<32>, t1);
    let res_hi = map3!(_mm256_add_epi64, t2, t1_hi);

    // Form res_lo by combining the low half of mul_ll with the low half of t1 (shifted into high
    // position).
    let t1_lo = {
        let t1_ps = map3!(_mm256_castsi256_ps, t1);
        let t1_lo_ps = map3!(_mm256_moveldup_ps, t1_ps);
        map3!(_mm256_castps_si256, t1_lo_ps)
    };
    let res_lo = map3!(_mm256_blend_epi32::<0xaa>, mul_ll, t1_lo);

    (res_lo, res_hi)
}

/// Addition, where the second operand is `0 <= y < 0xffffffff00000001`.
#[inline(always)]
unsafe fn add_small(
    x_s: (__m256i, __m256i, __m256i),
    y: (__m256i, __m256i, __m256i),
) -> (__m256i, __m256i, __m256i) {
    let res_wrapped_s = map3!(_mm256_add_epi64, x_s, y);
    let mask = map3!(_mm256_cmpgt_epi32, x_s, res_wrapped_s);
    let wrapback_amt = map3!(_mm256_srli_epi64::<32>, mask); // EPSILON if overflowed else 0.
    let res_s = map3!(_mm256_add_epi64, res_wrapped_s, wrapback_amt);
    res_s
}

#[inline(always)]
unsafe fn maybe_adj_sub(res_wrapped_s: __m256i, mask: __m256i) -> __m256i {
    // The subtraction is very unlikely to overflow so we're best off branching.
    // The even u32s in `mask` are meaningless, so we want to ignore them. `_mm256_testz_pd`
    // branches depending on the sign bit of double-precision (64-bit) floats. Bit cast `mask` to
    // floating-point (this is free).
    let mask_pd = _mm256_castsi256_pd(mask);
    // `_mm256_testz_pd(mask_pd, mask_pd) == 1` iff all sign bits are 0, meaning that underflow
    // did not occur for any of the vector elements.
    if _mm256_testz_pd(mask_pd, mask_pd) == 1 {
        res_wrapped_s
    } else {
        branch_hint();
        // Highly unlikely: underflow did occur. Find adjustment per element and apply it.
        let adj_amount = _mm256_srli_epi64::<32>(mask); // EPSILON if underflow.
        _mm256_sub_epi64(res_wrapped_s, adj_amount)
    }
}

/// Addition, where the second operand is much smaller than `0xffffffff00000001`.
#[inline(always)]
unsafe fn sub_tiny(
    x_s: (__m256i, __m256i, __m256i),
    y: (__m256i, __m256i, __m256i),
) -> (__m256i, __m256i, __m256i) {
    let res_wrapped_s = map3!(_mm256_sub_epi64, x_s, y);
    let mask = map3!(_mm256_cmpgt_epi32, res_wrapped_s, x_s);
    let res_s = map3!(maybe_adj_sub, res_wrapped_s, mask);
    res_s
}

#[inline(always)]
unsafe fn reduce3(
    (lo0, hi0): ((__m256i, __m256i, __m256i), (__m256i, __m256i, __m256i)),
) -> (__m256i, __m256i, __m256i) {
    let sign_bit = _mm256_set1_epi64x(i64::MIN);
    let epsilon = _mm256_set1_epi64x(0xffffffff);
    let lo0_s = map3!(_mm256_xor_si256, lo0, rep sign_bit);
    let hi_hi0 = map3!(_mm256_srli_epi64::<32>, hi0);
    let lo1_s = sub_tiny(lo0_s, hi_hi0);
    let t1 = map3!(_mm256_mul_epu32, hi0, rep epsilon);
    let lo2_s = add_small(lo1_s, t1);
    let lo2 = map3!(_mm256_xor_si256, lo2_s, rep sign_bit);
    lo2
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

#[inline(always)]
unsafe fn mds_multiply_and_add_round_const_s(
    state: (__m256i, __m256i, __m256i),
    (base, index): (*const u64, usize),
) -> ((__m256i, __m256i, __m256i), (__m256i, __m256i, __m256i)) {
    // TODO: Would it be faster to save the input to memory and do unaligned
    //   loads instead of swizzling? It would reduce pressure on port 5 but it
    //   would also have high latency (no store forwarding).
    // TODO: Would it be faster to store the lo and hi inputs and outputs on one
    //   vector? I.e., we currently operate on [lo(s[0]), lo(s[1]), lo(s[2]),
    //   lo(s[3])] and [hi(s[0]), hi(s[1]), hi(s[2]), hi(s[3])] separately. Using
    //   [lo(s[0]), lo(s[1]), hi(s[0]), hi(s[1])] and [lo(s[2]), lo(s[3]),
    //   hi(s[2]), hi(s[3])] would save us a few swizzles but would also need more
    //   registers.
    // TODO: Plain-vanilla matrix-vector multiplication might also work. We take
    //   one element of the input (a scalar), multiply a column by it, and
    //   accumulate. It would require shifts by amounts loaded from memory, but
    //   would eliminate all swizzles. The downside is that we can no longer
    //   special-case MDS == 0 and MDS == 1, so we end up with more shifts.
    // TODO: Building on the above: FMA? It has high latency (4 cycles) but we
    //   have enough operands to mask it. The main annoyance will be conversion
    //   to/from floating-point.
    // TODO: Try taking the complex Fourier transform and doing the convolution
    //   with elementwise Fourier multiplication. Alternatively, try a Fourier
    //   transform modulo Q, such that the prime field fits the result without
    //   wraparound (i.e. Q > 0x1_1536_fffe_eac9) and has fast multiplication/-
    //   reduction.

    // At the end of the matrix-vector multiplication r = Ms,
    // - ymm3 holds r[0:4]
    // - ymm4 holds r[4:8]
    // - ymm5 holds r[8:12]
    // - ymm6 holds r[2:6]
    // - ymm7 holds r[6:10]
    // - ymm8 holds concat(r[10:12], r[0:2])
    // Note that there are duplicates. E.g. r[0] is represented by ymm3[0] and
    // ymm8[2]. To obtain the final result, we must sum the duplicate entries:
    //   ymm3[0:2] += ymm8[2:4]
    //   ymm3[2:4] += ymm6[0:2]
    //   ymm4[0:2] += ymm6[2:4]
    //   ymm4[2:4] += ymm7[0:2]
    //   ymm5[0:2] += ymm7[2:4]
    //   ymm5[2:4] += ymm8[0:2]
    // Thus, the final result resides in ymm3, ymm4, ymm5.

    // WARNING: This code assumes that sum(1 << exp for exp in MDS_EXPS) * 0xffffffff fits in a
    // u64. If this guarantee ceases to hold, then it will no longer be correct.
    let (unreduced_lo0_s, unreduced_lo1_s, unreduced_lo2_s): (__m256i, __m256i, __m256i);
    let (unreduced_hi0, unreduced_hi1, unreduced_hi2): (__m256i, __m256i, __m256i);
    let epsilon = _mm256_set1_epi64x(0xffffffff);
    asm!(
        // Extract low 32 bits of the word
        "vpand ymm9,  ymm14, ymm0",
        "vpand ymm10, ymm14, ymm1",
        "vpand ymm11, ymm14, ymm2",

        "mov eax, 1",

        // Fall through for MDS matrix multiplication on low 32 bits

        // This is a GCC _local label_. For details, see
        // https://doc.rust-lang.org/rust-by-example/unsafe/asm.html#labels
        // In short, the assembler makes sure to assign a unique name to replace `2:` with a unique
        // name, so the label does not clash with any compiler-generated label. `2:` can appear
        // multiple times; to disambiguate, we must refer to it as `2b` or `2f`, specifying the
        // direction as _backward_ or _forward_.
        "2:",
        // NB: This block is run twice: once on the low 32 bits and once for the
        // high 32 bits. The 32-bit -> 64-bit matrix multiplication is responsible
        // for the majority of the instructions in this routine. By reusing them,
        // we decrease the burden on instruction caches by over one third.

        // 32-bit -> 64-bit MDS matrix multiplication
        // The scalar loop goes:
        //     for r in 0..WIDTH {
        //         let mut res = 0u128;
        //         for i in 0..WIDTH {
        //             res += (state[(i + r) % WIDTH] as u128) << MDS_MATRIX_EXPS[i];
        //         }
        //         result[r] = reduce(res);
        //     }
        //
        // Here, we swap the loops. Equivalent to:
        //     let mut res = [0u128; WIDTH];
        //     for i in 0..WIDTH {
        //         let mds_matrix_exp = MDS_MATRIX_EXPS[i];
        //         for r in 0..WIDTH {
        //             res[r] += (state[(i + r) % WIDTH] as u128) << mds_matrix_exp;
        //         }
        //     }
        //     for r in 0..WIDTH {
        //         result[r] = reduce(res[r]);
        //     }
        //
        // Notice that in the lower version, all iterations of the inner loop
        // shift by the same amount. In vector, we perform multiple iterations of
        // the loop at once, and vector shifts are cheaper when all elements are
        // shifted by the same amount.
        //
        // We use a trick to avoid rotating the state vector many times. We
        // have as input the state vector and the state vector rotated by one. We
        // also have two accumulators: an unrotated one and one that's rotated by
        // two. Rotations by three are achieved by matching an input rotated by
        // one with an accumulator rotated by two. Rotations by four are free:
        // they are done by using a different register.

        // mds[0 - 0] = 0 not done; would be a move from in0 to ymm3
        // ymm3 not set
        // mds[0 - 4] = 12
        "vpsllq ymm4, ymm9, 12",
        // mds[0 - 8] = 3
        "vpsllq ymm5, ymm9, 3",
        // mds[0 - 2] = 16
        "vpsllq ymm6, ymm9, 16",
        // mds[0 - 6] = mds[0 - 10] = 1
        "vpaddq ymm7, ymm9, ymm9",
        // ymm8 not written
        // ymm3 and ymm8 have not been written to, because those would be unnecessary
        // copies. Implicitly, ymm3 := in0 and ymm8 := ymm7.

        // ymm12 := [ymm9[1], ymm9[2], ymm9[3], ymm10[0]]
        "vperm2i128 ymm13, ymm9, ymm10, 0x21",
        "vshufpd    ymm12, ymm9, ymm13, 0x5",

        // ymm3 and ymm8 are not read because they have not been written to
        // earlier. Instead, the "current value" of ymm3 is read from ymm9 and the
        // "current value" of ymm8 is read from ymm7.
        // mds[4 - 0] = 3
        "vpsllq ymm13, ymm10, 3",
        "vpaddq ymm3,  ymm9, ymm13",
        // mds[4 - 4] = 0
        "vpaddq ymm4,  ymm4, ymm10",
        // mds[4 - 8] = 12
        "vpsllq ymm13, ymm10, 12",
        "vpaddq ymm5,  ymm5, ymm13",
        // mds[4 - 2] = mds[4 - 10] = 1
        "vpaddq ymm13, ymm10, ymm10",
        "vpaddq ymm6,  ymm6, ymm13",
        "vpaddq ymm8,  ymm7, ymm13",
        // mds[4 - 6] = 16
        "vpsllq ymm13, ymm10, 16",
        "vpaddq ymm7,  ymm7, ymm13",

        // mds[1 - 0] = 0
        "vpaddq ymm3,  ymm3, ymm12",
        // mds[1 - 4] = 3
        "vpsllq ymm13, ymm12, 3",
        "vpaddq ymm4,  ymm4, ymm13",
        // mds[1 - 8] = 5
        "vpsllq ymm13, ymm12, 5",
        "vpaddq ymm5,  ymm5, ymm13",
        // mds[1 - 2] = 10
        "vpsllq ymm13, ymm12, 10",
        "vpaddq ymm6,  ymm6, ymm13",
        // mds[1 - 6] = 8
        "vpsllq ymm13, ymm12, 8",
        "vpaddq ymm7,  ymm7, ymm13",
        // mds[1 - 10] = 0
        "vpaddq ymm8, ymm8, ymm12",

        // ymm10 := [ymm10[1], ymm10[2], ymm10[3], ymm11[0]]
        "vperm2i128 ymm13, ymm10, ymm11, 0x21",
        "vshufpd    ymm10, ymm10, ymm13, 0x5",

        // mds[8 - 0] = 12
        "vpsllq ymm13, ymm11, 12",
        "vpaddq ymm3,  ymm3, ymm13",
        // mds[8 - 4] = 3
        "vpsllq ymm13, ymm11, 3",
        "vpaddq ymm4,  ymm4, ymm13",
        // mds[8 - 8] = 0
        "vpaddq ymm5,  ymm5, ymm11",
        // mds[8 - 2] = mds[8 - 6] = 1
        "vpaddq ymm13, ymm11, ymm11",
        "vpaddq ymm6,  ymm6, ymm13",
        "vpaddq ymm7,  ymm7, ymm13",
        // mds[8 - 10] = 16
        "vpsllq ymm13, ymm11, 16",
        "vpaddq ymm8,  ymm8, ymm13",

        // ymm9 := [ymm11[1], ymm11[2], ymm11[3], ymm9[0]]
        "vperm2i128 ymm13, ymm11, ymm9, 0x21",
        "vshufpd    ymm9,  ymm11, ymm13, 0x5",

        // mds[5 - 0] = 5
        "vpsllq ymm13, ymm10, 5",
        "vpaddq ymm3,  ymm3, ymm13",
        // mds[5 - 4] = 0
        "vpaddq ymm4,  ymm4, ymm10",
        // mds[5 - 8] = 3
        "vpsllq ymm13, ymm10, 3",
        "vpaddq ymm5,  ymm5, ymm13",
        // mds[5 - 2] = 0
        "vpaddq ymm6,  ymm6, ymm10",
        // mds[5 - 6] = 10
        "vpsllq ymm13, ymm10, 10",
        "vpaddq ymm7,  ymm7, ymm13",
        // mds[5 - 10] = 8
        "vpsllq ymm13, ymm10, 8",
        "vpaddq ymm8,  ymm8, ymm13",

        // mds[9 - 0] = 3
        "vpsllq ymm13, ymm9, 3",
        "vpaddq ymm3,  ymm3, ymm13",
        // mds[9 - 4] = 5
        "vpsllq ymm13, ymm9, 5",
        "vpaddq ymm4,  ymm4, ymm13",
        // mds[9 - 8] = 0
        "vpaddq ymm5,  ymm5, ymm9",
        // mds[9 - 2] = 8
        "vpsllq ymm13, ymm9, 8",
        "vpaddq ymm6,  ymm6, ymm13",
        // mds[9 - 6] = 0
        "vpaddq ymm7,  ymm7, ymm9",
        // mds[9 - 10] = 10
        "vpsllq ymm13, ymm9, 10",
        "vpaddq ymm8,  ymm8, ymm13",

        // Rotate ymm6-ymm8 and add to the corresponding elements of ymm3-ymm5
        "vperm2i128 ymm13, ymm8, ymm6, 0x21",
        "vpaddq     ymm3,  ymm3, ymm13",
        "vperm2i128 ymm13, ymm6, ymm7, 0x21",
        "vpaddq     ymm4,  ymm4, ymm13",
        "vperm2i128 ymm13, ymm7, ymm8, 0x21",
        "vpaddq     ymm5,  ymm5, ymm13",

        // If this is the first time we have run 2: (low 32 bits) then continue.
        // If second time (high 32 bits), then jump to 3:.
        "dec eax",
        // Jump to the _local label_ (see above) `3:`. `f` for _forward_ specifies the direction.
        "jnz 3f",

        // Extract high 32 bits
        "vpsrlq ymm9,  ymm0, 32",
        "vpsrlq ymm10, ymm1, 32",
        "vpsrlq ymm11, ymm2, 32",

        // Need to move the low result from ymm3-ymm5 to ymm0-13 so it is not
        // overwritten. Save three instructions by combining the move with the constant layer,
        // which would otherwise be done in 3:. The round constants include the shift by 2**63, so
        // the resulting ymm0,1,2 are also shifted by 2**63.
        // It is safe to add the round constants here without checking for overflow. The values in
        // ymm3,4,5 are guaranteed to be <= 0x11536fffeeac9. All round constants are < 2**64
        // - 0x11536fffeeac9.
        // WARNING: If this guarantee ceases to hold due to a change in the MDS matrix or round
        // constants, then this code will no longer be correct.
        "vpaddq ymm0, ymm3, [{base} + {index}]",
        "vpaddq ymm1, ymm4, [{base} + {index} + 32]",
        "vpaddq ymm2, ymm5, [{base} + {index} + 64]",

        // MDS matrix multiplication, again. This time on high 32 bits.
        // Jump to the _local label_ (see above) `2:`. `b` for _backward_ specifies the direction.
        "jmp 2b",

        // `3:` is a _local label_ (see above).
        "3:",
        // Just done the MDS matrix multiplication on high 32 bits.
        // The high results are in ymm3, ymm4, ymm5.
        // The low results (shifted by 2**63 and including the following constant layer) are in
        // ymm0, ymm1, ymm2.
        base = in(reg) base,
        index = in(reg) index,
        inout("ymm0") state.0 => unreduced_lo0_s,
        inout("ymm1") state.1 => unreduced_lo1_s,
        inout("ymm2") state.2 => unreduced_lo2_s,
        out("ymm3") unreduced_hi0,
        out("ymm4") unreduced_hi1,
        out("ymm5") unreduced_hi2,
        out("ymm6") _,out("ymm7") _, out("ymm8") _, out("ymm9") _,
        out("ymm10") _, out("ymm11") _, out("ymm12") _, out("ymm13") _,
        in("ymm14") epsilon,
        out("rax") _,
        options(pure, nomem, nostack),
    );
    (
        (unreduced_lo0_s, unreduced_lo1_s, unreduced_lo2_s),
        (unreduced_hi0, unreduced_hi1, unreduced_hi2),
    )
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
        "vmovdqu ymm6, [{mds_matrix}]",
        "vmovdqu ymm7, [{mds_matrix} + 32]",
        "vmovdqu ymm8, [{mds_matrix} + 64]",
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
        mds_matrix = in(reg) &TOP_ROW_EXPS,
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

#[inline(always)]
unsafe fn load_state(state: &[GoldilocksField; 12]) -> (__m256i, __m256i, __m256i) {
    (
        _mm256_loadu_si256((&state[0..4]).as_ptr().cast::<__m256i>()),
        _mm256_loadu_si256((&state[4..8]).as_ptr().cast::<__m256i>()),
        _mm256_loadu_si256((&state[8..12]).as_ptr().cast::<__m256i>()),
    )
}

#[inline(always)]
unsafe fn store_state(buf: &mut [GoldilocksField; 12], state: (__m256i, __m256i, __m256i)) {
    _mm256_storeu_si256((&mut buf[0..4]).as_mut_ptr().cast::<__m256i>(), state.0);
    _mm256_storeu_si256((&mut buf[4..8]).as_mut_ptr().cast::<__m256i>(), state.1);
    _mm256_storeu_si256((&mut buf[8..12]).as_mut_ptr().cast::<__m256i>(), state.2);
}

#[inline]
pub unsafe fn poseidon(state: &[GoldilocksField; 12]) -> [GoldilocksField; 12] {
    let state = load_state(state);

    // The first constant layer must be done explicitly. The remaining constant layers are fused
    // with the preceding MDS layer.
    let state = const_layer(state, &ALL_ROUND_CONSTANTS[0..WIDTH].try_into().unwrap());

    let state = half_full_rounds(state, 0);
    let state = all_partial_rounds(state, HALF_N_FULL_ROUNDS);
    let state = half_full_rounds(state, HALF_N_FULL_ROUNDS + N_PARTIAL_ROUNDS);

    let mut res = [GoldilocksField::ZERO; 12];
    store_state(&mut res, state);
    res
}

#[inline(always)]
pub unsafe fn constant_layer(state_arr: &mut [GoldilocksField; WIDTH], round_ctr: usize) {
    let state = load_state(state_arr);
    let round_consts = &ALL_ROUND_CONSTANTS[WIDTH * round_ctr..][..WIDTH]
        .try_into()
        .unwrap();
    let state = const_layer(state, round_consts);
    store_state(state_arr, state);
}

#[inline(always)]
pub unsafe fn sbox_layer(state_arr: &mut [GoldilocksField; WIDTH]) {
    let state = load_state(state_arr);
    let state = sbox_layer_full(state);
    store_state(state_arr, state);
}

#[inline(always)]
pub unsafe fn mds_layer(state: &[GoldilocksField; WIDTH]) -> [GoldilocksField; WIDTH] {
    let state = load_state(state);
    // We want to do an MDS layer without the constant layer.
    // The FUSED_ROUND_CONSTANTS for the last round are all 0 (shifted by 2**63 as required).
    let round_consts = FUSED_ROUND_CONSTANTS[WIDTH * (N_ROUNDS - 1)..].as_ptr();
    let state = mds_const_layers_full(state, (round_consts, 0));
    let mut res = [GoldilocksField::ZERO; 12];
    store_state(&mut res, state);
    res
}
