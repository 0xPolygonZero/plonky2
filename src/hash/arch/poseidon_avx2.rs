use core::arch::x86_64::*;
use std::mem::size_of;

use crate::field::field_types::Field;
use crate::field::goldilocks_field::GoldilocksField;
use crate::hash::poseidon::{Poseidon, ALL_ROUND_CONSTANTS, HALF_N_FULL_ROUNDS, N_PARTIAL_ROUNDS};

const WIDTH: usize = 12;

// This is the top row of the MDS matrix. Concretely, it's the MDS exps vector at the following
// indices: [0, 11, ..., 1].
static TOP_ROW_EXPS: [usize; 12] = [0, 10, 16, 3, 12, 8, 1, 5, 3, 0, 1, 0];

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

#[inline(always)]
unsafe fn const_layer(
    (state0_s, state1_s, state2_s): (__m256i, __m256i, __m256i),
    (base, index): (*const GoldilocksField, usize),
) -> (__m256i, __m256i, __m256i) {
    // TODO: We can make this entire layer effectively free by folding it into MDS multiplication.
    let (state0, state1, state2): (__m256i, __m256i, __m256i);
    let sign_bit = _mm256_set1_epi64x(i64::MIN);
    asm!(
        // Below is optimized for latency. In particular, we avoid pcmpgtq because it has latency
        // of 3 cycles and can only run on port 5. pcmpgtd is much faster.
        "vpaddq    {t0}, {state0}, [{base:r} + {index:r}]",
        "vpaddq    {t1}, {state1}, [{base:r} + {index:r} + 32]",
        "vpaddq    {t2}, {state2}, [{base:r} + {index:r} + 64]",
        // It's okay to do vpcmpgtd (instead of vpcmpgtq) because all the round
        // constants are >= 1 << 32 and < field order.
        "vpcmpgtd  {u0}, {state0}, {t0}",
        "vpcmpgtd  {u1}, {state1}, {t1}",
        "vpcmpgtd  {u2}, {state2}, {t2}",
        // Unshift by 1 << 63.
        "vpxor     {t0}, {sign_bit}, {t0}",
        "vpxor     {t1}, {sign_bit}, {t1}",
        "vpxor     {t2}, {sign_bit}, {t2}",
        // Add epsilon if t >> 32 > state >> 32.
        "vpsrlq    {u0}, {u0}, 32",
        "vpsrlq    {u1}, {u1}, 32",
        "vpsrlq    {u2}, {u2}, 32",
        "vpaddq    {state0}, {u0}, {t0}",
        "vpaddq    {state1}, {u1}, {t1}",
        "vpaddq    {state2}, {u2}, {t2}",

        state0 = inout(ymm_reg) state0_s => state0,
        state1 = inout(ymm_reg) state1_s => state1,
        state2 = inout(ymm_reg) state2_s => state2,
        t0 = out(ymm_reg) _, t1 = out(ymm_reg) _, t2 = out(ymm_reg) _,
        u0 = out(ymm_reg) _, u1 = out(ymm_reg) _, u2 = out(ymm_reg) _,
        sign_bit = in(ymm_reg) sign_bit,
        base = in(reg) base,
        index = in(reg) index,
        options(pure, readonly, preserves_flags, nostack),
    );
    (state0, state1, state2)
}

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
unsafe fn mds_layer_reduce_s(
    lo_s: (__m256i, __m256i, __m256i),
    hi: (__m256i, __m256i, __m256i),
) -> (__m256i, __m256i, __m256i) {
    // This is done in assembly because, frankly, it's cleaner than intrinsics. We also don't have
    // to worry about whether the compiler is doing weird things. This entire routine needs proper
    // pipelining so there's no point rewriting this, only to have to rewrite it again.
    let res0_s: __m256i;
    let res1_s: __m256i;
    let res2_s: __m256i;
    let epsilon = _mm256_set1_epi64x(0xffffffff);
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
        "vpaddq   ymm0, ymm6, ymm3",
        "vpaddq   ymm1, ymm7, ymm4",
        "vpaddq   ymm2, ymm8, ymm5",
        inout("ymm0") lo_s.0 => res0_s,
        inout("ymm1") lo_s.1 => res1_s,
        inout("ymm2") lo_s.2 => res2_s,
        inout("ymm3") hi.0 => _,
        inout("ymm4") hi.1 => _,
        inout("ymm5") hi.2 => _,
        out("ymm6") _, out("ymm7") _, out("ymm8") _, out("ymm9") _, out("ymm10") _, out("ymm11") _,
        in("ymm14") epsilon,
        options(pure, nomem, preserves_flags, nostack),
    );
    (res0_s, res1_s, res2_s)
}

#[inline(always)]
unsafe fn mds_layer_multiply_s(
    state: (__m256i, __m256i, __m256i),
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
    let (unreduced_lo0_s, unreduced_lo1_s, unreduced_lo2_s): (__m256i, __m256i, __m256i);
    let (unreduced_hi0, unreduced_hi1, unreduced_hi2): (__m256i, __m256i, __m256i);
    let sign_bit = _mm256_set1_epi64x(i64::MIN);
    let epsilon = _mm256_set1_epi64x(0xffffffff);
    asm!(
        // Extract low 32 bits of the word
        "vpand ymm9,  ymm14, ymm0",
        "vpand ymm10, ymm14, ymm1",
        "vpand ymm11, ymm14, ymm2",

        "mov eax, 1",

        // Fall through for MDS matrix multiplication on low 32 bits

        // This is a GCC _local label_. For details, see
        // https://doc.rust-lang.org/beta/unstable-book/library-features/asm.html#labels
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
        // overwritten. Save three instructions by combining the move with xor ymm15,
        // which would otherwise be done in 3:.
        "vpxor ymm0, ymm15, ymm3",
        "vpxor ymm1, ymm15, ymm4",
        "vpxor ymm2, ymm15, ymm5",

        // MDS matrix multiplication, again. This time on high 32 bits.
        // Jump to the _local label_ (see above) `2:`. `b` for _backward_ specifies the direction.
        "jmp 2b",

        // `3:` is a _local label_ (see above).
        "3:",
        // Just done the MDS matrix multiplication on high 32 bits.
        // The high results are in ymm3, ymm4, ymm5.
        // The low results (shifted by 2**63) are in ymm0, ymm1, ymm2
        inout("ymm0") state.0 => unreduced_lo0_s,
        inout("ymm1") state.1 => unreduced_lo1_s,
        inout("ymm2") state.2 => unreduced_lo2_s,
        out("ymm3") unreduced_hi0,
        out("ymm4") unreduced_hi1,
        out("ymm5") unreduced_hi2,
        out("ymm6") _,out("ymm7") _, out("ymm8") _, out("ymm9") _,
        out("ymm10") _, out("ymm11") _, out("ymm12") _, out("ymm13") _,
        in("ymm14") epsilon, in("ymm15") sign_bit,
        out("rax") _,
        options(pure, nomem, nostack),
    );
    (
        (unreduced_lo0_s, unreduced_lo1_s, unreduced_lo2_s),
        (unreduced_hi0, unreduced_hi1, unreduced_hi2),
    )
}

#[inline(always)]
unsafe fn mds_layer_full_s(state: (__m256i, __m256i, __m256i)) -> (__m256i, __m256i, __m256i) {
    let (unreduced_lo_s, unreduced_hi) = mds_layer_multiply_s(state);
    mds_layer_reduce_s(unreduced_lo_s, unreduced_hi)
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
unsafe fn sbox_mds_layers_partial_s(
    (state0, state1, state2): (__m256i, __m256i, __m256i),
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
    ) = mds_layer_multiply_s((state0bcd, state1, state2));
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
    mds_layer_reduce_s(
        (unreduced_lo0_s, unreduced_lo1_s, unreduced_lo2_s),
        (unreduced_hi0, unreduced_hi1, unreduced_hi2),
    )
}

#[inline(always)]
unsafe fn full_round_s(
    state_s: (__m256i, __m256i, __m256i),
    round_constants: (*const GoldilocksField, usize),
) -> (__m256i, __m256i, __m256i) {
    let state = const_layer(state_s, round_constants);
    let state = sbox_layer_full(state);
    let state_s = mds_layer_full_s(state);
    state_s
}

#[inline(always)]
unsafe fn partial_round_s(
    state_s: (__m256i, __m256i, __m256i),
    round_constants: (*const GoldilocksField, usize),
) -> (__m256i, __m256i, __m256i) {
    let state = const_layer(state_s, round_constants);
    let state_s = sbox_mds_layers_partial_s(state);
    state_s
}

#[inline] // Called twice; permit inlining but don't _require_ it
unsafe fn half_full_rounds_s(
    mut state_s: (__m256i, __m256i, __m256i),
    start_round: usize,
) -> (__m256i, __m256i, __m256i) {
    let base = (&ALL_ROUND_CONSTANTS
        [WIDTH * start_round..WIDTH * start_round + WIDTH * HALF_N_FULL_ROUNDS])
        .as_ptr()
        .cast::<GoldilocksField>();

    for i in 0..HALF_N_FULL_ROUNDS {
        state_s = full_round_s(state_s, (base, i * WIDTH * size_of::<u64>()));
    }
    state_s
}

#[inline(always)]
unsafe fn all_partial_rounds_s(
    mut state_s: (__m256i, __m256i, __m256i),
    start_round: usize,
) -> (__m256i, __m256i, __m256i) {
    let base = (&ALL_ROUND_CONSTANTS
        [WIDTH * start_round..WIDTH * start_round + WIDTH * N_PARTIAL_ROUNDS])
        .as_ptr()
        .cast::<GoldilocksField>();

    for i in 0..N_PARTIAL_ROUNDS {
        state_s = partial_round_s(state_s, (base, i * WIDTH * size_of::<u64>()));
    }
    state_s
}

#[inline]
pub unsafe fn poseidon(state: &[GoldilocksField; 12]) -> [GoldilocksField; 12] {
    let sign_bit = _mm256_set1_epi64x(i64::MIN);

    let mut s0 = _mm256_loadu_si256((&state[0..4]).as_ptr().cast::<__m256i>());
    let mut s1 = _mm256_loadu_si256((&state[4..8]).as_ptr().cast::<__m256i>());
    let mut s2 = _mm256_loadu_si256((&state[8..12]).as_ptr().cast::<__m256i>());
    s0 = _mm256_xor_si256(s0, sign_bit);
    s1 = _mm256_xor_si256(s1, sign_bit);
    s2 = _mm256_xor_si256(s2, sign_bit);

    (s0, s1, s2) = half_full_rounds_s((s0, s1, s2), 0);
    (s0, s1, s2) = all_partial_rounds_s((s0, s1, s2), HALF_N_FULL_ROUNDS);
    (s0, s1, s2) = half_full_rounds_s((s0, s1, s2), HALF_N_FULL_ROUNDS + N_PARTIAL_ROUNDS);

    s0 = _mm256_xor_si256(s0, sign_bit);
    s1 = _mm256_xor_si256(s1, sign_bit);
    s2 = _mm256_xor_si256(s2, sign_bit);

    let mut res = [GoldilocksField::ZERO; 12];
    _mm256_storeu_si256((&mut res[0..4]).as_mut_ptr().cast::<__m256i>(), s0);
    _mm256_storeu_si256((&mut res[4..8]).as_mut_ptr().cast::<__m256i>(), s1);
    _mm256_storeu_si256((&mut res[8..12]).as_mut_ptr().cast::<__m256i>(), s2);

    res
}
