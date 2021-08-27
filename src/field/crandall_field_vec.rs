use core::arch::x86_64::*;

const EPSILON: u64 = (1 << 31) + (1 << 28) - 1;
const FIELD_ORDER: u64 = 0u64.overflowing_sub(EPSILON).0;
const SIGN_BIT: u64 = 1 << 63;

unsafe fn field_order() -> __m256i {
    _mm256_set1_epi64x(FIELD_ORDER as i64)
}

unsafe fn epsilon() -> __m256i {
    _mm256_set1_epi64x(EPSILON as i64)
}

unsafe fn sign_bit() -> __m256i {
    _mm256_set1_epi64x(SIGN_BIT as i64)
}

// Resources:
// 1. Intel Intrinsics Guide for explanation of each intrinsic:
//    https://software.intel.com/sites/landingpage/IntrinsicsGuide/
// 2. uops.info lists micro-ops for each instruction: https://uops.info/table.html
// 3. Intel optimization manual for introduction to x86 vector extensions and best practices:
//    https://software.intel.com/content/www/us/en/develop/download/intel-64-and-ia-32-architectures-optimization-reference-manual.html

// Preliminary knowledge:
// 1. Vector code usually avoids branching. Instead of branches, we can do input selection with
//    _mm256_blendv_epi8 or similar instruction. If all we're doing is conditionally zeroing a
//    vector element then _mm256_and_si256 or _mm256_andnot_si256 may be used and are cheaper.
//
// 2. AVX does not support addition with carry but 128-bit (2-word) addition can be easily
//    emulated. The method recognizes that for a + b overflowed iff (a + b) < a:
//        i. res_lo = a_lo + b_lo
//       ii. carry_mask = res_lo < a_lo
//      iii. res_hi = a_hi + b_hi - carry_mask
//    Notice that carry_mask is subtracted, not added. This is because AVX comparison instructions
//    return -1 (all bits 1) for true and 0 for false.
//
// 3. AVX does not have unsigned 64-bit comparisons. Those can be emulated with signed comparisons
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

/// Add 2^63 with overflow. Needed to emulate unsigned comparisons (see point 3. above).
unsafe fn shift(x: __m256i) -> __m256i {
    _mm256_xor_si256(x, sign_bit())
}

/// Convert to canonical representation.
/// The argument is assumed to be shifted by 1 << 63 (i.e. x_s = x + 1<<63, where x is the
///   Crandall field value). The returned value is similarly shifted by 1 << 63 (i.e. we return y_s
///   = y + 1<<63, where 0 <= y < FIELD_ORDER).
unsafe fn canonicalize_s(x_s: __m256i) -> __m256i {
    // If x >= FIELD_ORDER then corresponding mask bits are all 0; otherwise all 1.
    let mask = _mm256_cmpgt_epi64(shift(field_order()), x_s);
    // wrapback_amt is -FIELD_ORDER if mask is 0; otherwise 0.
    let wrapback_amt = _mm256_andnot_si256(mask, epsilon());
    _mm256_add_epi64(x_s, wrapback_amt)
}

// Theoretical throughput
// Scalar version (compiled): 1.75 cycles/word
// Scalar version (optimized asm): 1 cycle/word
// Below (128-bit vectors): 1.5 cycles/word
// Below (256-bit vectors): .75 cycles/word
pub unsafe fn add(x: __m256i, y: __m256i) -> __m256i {
    let mut y_s = shift(y);
    y_s = canonicalize_s(y_s);
    let res_wrapped_s = _mm256_add_epi64(x, y_s);
    let mask = _mm256_cmpgt_epi64(y_s, res_wrapped_s); // 1 if overflowed else 0.
    let wrapback_amt = _mm256_and_si256(mask, epsilon()); // -FIELD_ORDER if overflowed else 0.
    let res_s = _mm256_add_epi64(res_wrapped_s, wrapback_amt);
    shift(res_s)
}

// Theoretical throughput
// Scalar version (compiled): 1.75 cycles/word
// Scalar version (optimized asm): 1 cycle/word
// Below (128-bit vectors): 1.5 cycles/word
// Below (256-bit vectors): .75 cycles/word
pub unsafe fn sub(x: __m256i, y: __m256i) -> __m256i {
    let mut y_s = shift(y);
    y_s = canonicalize_s(y_s);
    let x_s = shift(x);
    let mask = _mm256_cmpgt_epi64(y_s, x_s); // 1 if sub will underflow (y > y) else 0.
    let wrapback_amt = _mm256_and_si256(mask, epsilon()); // -FIELD_ORDER if underflow else 0.
    let res_wrapped = _mm256_sub_epi64(x_s, y_s);
    let res = _mm256_sub_epi64(res_wrapped, wrapback_amt);
    res
}

/// Full 64-bit by 64-bit multiplication. This emulated multiplication is 1.5x slower than the
/// scalar instruction, but may be worth it if we want our data to live in vector registers.
unsafe fn mul64_64_s(x: __m256i, y: __m256i) -> (__m256i, __m256i) {
    let x_hi = _mm256_srli_epi64(x, 32);
    let y_hi = _mm256_srli_epi64(y, 32);
    let mul_ll = _mm256_mul_epu32(x, y);
    let mul_lh = _mm256_mul_epu32(x, y_hi);
    let mul_hl = _mm256_mul_epu32(x_hi, y);
    let mul_hh = _mm256_mul_epu32(x_hi, y_hi);

    let res_lo0_s = shift(mul_ll);
    let res_hi0 = mul_hh;

    let res_lo1_s = _mm256_add_epi32(res_lo0_s, _mm256_slli_epi64(mul_lh, 32));
    let res_hi1 = _mm256_sub_epi64(res_hi0, _mm256_cmpgt_epi64(res_lo0_s, res_lo1_s)); // Carry.

    let res_lo2_s = _mm256_add_epi32(res_lo1_s, _mm256_slli_epi64(mul_hl, 32));
    let res_hi2 = _mm256_sub_epi64(res_hi1, _mm256_cmpgt_epi64(res_lo1_s, res_lo2_s)); // Carry.

    let res_hi3 = _mm256_add_epi64(res_hi2, _mm256_srli_epi64(mul_lh, 32));
    let res_hi4 = _mm256_add_epi64(res_hi3, _mm256_srli_epi64(mul_hl, 32));

    (res_hi4, res_lo2_s)
}

/// u128 + u64 addition with carry. The second argument is pre-shifted by 2^63. The result is also
/// shifted.
unsafe fn add_with_carry128_64s_s(x: (__m256i, __m256i), y_s: __m256i) -> (__m256i, __m256i) {
    let (x_hi, x_lo) = x;
    let res_lo_s = _mm256_add_epi64(x_lo, y_s);
    let carry = _mm256_cmpgt_epi64(y_s, res_lo_s);
    let res_hi = _mm256_sub_epi64(x_hi, carry);
    (res_hi, res_lo_s)
}

/// u128 + u64 addition with carry. The first argument is pre-shifted by 2^63. The result is also
/// shifted.
unsafe fn add_with_carry128s_64_s(x_s: (__m256i, __m256i), y: __m256i) -> (__m256i, __m256i) {
    let (x_hi, x_lo_s) = x_s;
    let res_lo_s = _mm256_add_epi64(x_lo_s, y);
    let carry = _mm256_cmpgt_epi64(x_lo_s, res_lo_s);
    let res_hi = _mm256_sub_epi64(x_hi, carry);
    (res_hi, res_lo_s)
}

/// u64 * u32 + u64 fused multiply-add. The result is given as a tuple (u64, u64). The third
/// argument is assumed to be pre-shifted by 2^63. The result is similarly shifted.
unsafe fn fmadd_64_32_64s_s(x: __m256i, y: __m256i, z_s: __m256i) -> (__m256i, __m256i) {
    let x_hi = _mm256_srli_epi64(x, 32);
    let mul_lo = _mm256_mul_epu32(x, y);
    let mul_hi = _mm256_mul_epu32(x_hi, y);
    let tmp_s = add_with_carry128_64s_s(
        (_mm256_srli_epi64(mul_hi, 32), _mm256_slli_epi64(mul_hi, 32)),
        z_s,
    );
    add_with_carry128s_64_s(tmp_s, mul_lo)
}

/// Reduce a u128 modulo FIELD_ORDER. The input is (u64, u64), pre-shifted by 2^63. The result is
/// similarly shifted.
unsafe fn reduce128s_s(x_s: (__m256i, __m256i)) -> __m256i {
    let (hi0, lo0_s) = x_s;
    let (hi1, lo1_s) = fmadd_64_32_64s_s(hi0, epsilon(), lo0_s);
    let lo2 = _mm256_mul_epu32(hi1, epsilon());
    let res_wrapped_s = _mm256_add_epi64(lo1_s, lo2);
    let carry_mask = _mm256_cmpgt_epi64(lo1_s, res_wrapped_s); // all 1 if overflow
    let res_s = _mm256_add_epi64(res_wrapped_s, _mm256_and_si256(carry_mask, epsilon()));
    res_s
}

/// Multiply two integers modulo FIELD_ORDER.
pub unsafe fn mul(x: __m256i, y: __m256i) -> __m256i {
    shift(reduce128s_s(mul64_64_s(x, y)))
}

#[cfg(test)]
mod tests {
    use crate::field::crandall_field::CrandallField;
    use crate::field::crandall_field_vec::*;

    #[test]
    fn test_add() {
        let a: Vec<u64> = vec![
            14479013849828404771,
            9087029921428221768,
            2441288194761790662,
            5646033492608483824,
        ];
        let b: Vec<u64> = vec![
            17891926589593242302,
            11009798273260028228,
            2028722748960791447,
            7929433601095175579,
        ];

        let (res_v0, res_v1, res_v2, res_v3);
        unsafe {
            let av = _mm256_setr_epi64x(a[0] as i64, a[1] as i64, a[2] as i64, a[3] as i64);
            let bv = _mm256_setr_epi64x(b[0] as i64, b[1] as i64, b[2] as i64, b[3] as i64);
            let res_v = add(av, bv);

            res_v0 = CrandallField(_mm256_extract_epi64(res_v, 0) as u64);
            res_v1 = CrandallField(_mm256_extract_epi64(res_v, 1) as u64);
            res_v2 = CrandallField(_mm256_extract_epi64(res_v, 2) as u64);
            res_v3 = CrandallField(_mm256_extract_epi64(res_v, 3) as u64);
        }

        assert_eq!(res_v0, CrandallField(a[0]) + CrandallField(b[0]));
        assert_eq!(res_v1, CrandallField(a[1]) + CrandallField(b[1]));
        assert_eq!(res_v2, CrandallField(a[2]) + CrandallField(b[2]));
        assert_eq!(res_v3, CrandallField(a[3]) + CrandallField(b[3]));
    }

    #[test]
    fn test_sub() {
        let a: Vec<u64> = vec![
            16227227439196075598,
            6716317602924211287,
            2841323442296127245,
            7263852802381042972,
        ];
        let b: Vec<u64> = vec![
            12281116264768601762,
            6622018881932723404,
            8760433989034129451,
            15591286958183010066,
        ];

        let (res_v0, res_v1, res_v2, res_v3);
        unsafe {
            let av = _mm256_setr_epi64x(a[0] as i64, a[1] as i64, a[2] as i64, a[3] as i64);
            let bv = _mm256_setr_epi64x(b[0] as i64, b[1] as i64, b[2] as i64, b[3] as i64);
            let res_v = sub(av, bv);

            res_v0 = CrandallField(_mm256_extract_epi64(res_v, 0) as u64);
            res_v1 = CrandallField(_mm256_extract_epi64(res_v, 1) as u64);
            res_v2 = CrandallField(_mm256_extract_epi64(res_v, 2) as u64);
            res_v3 = CrandallField(_mm256_extract_epi64(res_v, 3) as u64);
        }

        assert_eq!(res_v0, CrandallField(a[0]) - CrandallField(b[0]));
        assert_eq!(res_v1, CrandallField(a[1]) - CrandallField(b[1]));
        assert_eq!(res_v2, CrandallField(a[2]) - CrandallField(b[2]));
        assert_eq!(res_v3, CrandallField(a[3]) - CrandallField(b[3]));
    }

    #[test]
    fn test_mul() {
        let a: Vec<u64> = vec![
            6809398725176840431,
            4482334701449839908,
            6005174454137817907,
            2577767633704502274,
        ];
        let b: Vec<u64> = vec![
            17152612508452320018,
            6114367496342734588,
            17057706769499534236,
            5548681120235370693,
        ];

        let (res_v0, res_v1, res_v2, res_v3);
        unsafe {
            let av = _mm256_setr_epi64x(a[0] as i64, a[1] as i64, a[2] as i64, a[3] as i64);
            let bv = _mm256_setr_epi64x(b[0] as i64, b[1] as i64, b[2] as i64, b[3] as i64);
            let res_v = mul(av, bv);

            res_v0 = CrandallField(_mm256_extract_epi64(res_v, 0) as u64);
            res_v1 = CrandallField(_mm256_extract_epi64(res_v, 1) as u64);
            res_v2 = CrandallField(_mm256_extract_epi64(res_v, 2) as u64);
            res_v3 = CrandallField(_mm256_extract_epi64(res_v, 3) as u64);
        }

        assert_eq!(res_v0, CrandallField(a[0]) * CrandallField(b[0]));
        assert_eq!(res_v1, CrandallField(a[1]) * CrandallField(b[1]));
        assert_eq!(res_v2, CrandallField(a[2]) * CrandallField(b[2]));
        assert_eq!(res_v3, CrandallField(a[3]) * CrandallField(b[3]));
    }
}
