use core::arch::x86_64::*;

use crate::field::crandall_field::CrandallField;
use crate::field::packed_avx2::common::{add_no_canonicalize_64_64s_s, epsilon, ReducibleAVX2};

/// (u64 << 64) + u64 + u64 -> u128 addition with carry. The third argument is pre-shifted by 2^63.
/// The result is also shifted.
#[inline]
unsafe fn add_with_carry_hi_lo_los_s(
    hi: __m256i,
    lo0: __m256i,
    lo1_s: __m256i,
) -> (__m256i, __m256i) {
    let res_lo_s = _mm256_add_epi64(lo0, lo1_s);
    // carry is -1 if overflow (res_lo < lo1) because cmpgt returns -1 on true and 0 on false.
    let carry = _mm256_cmpgt_epi64(lo1_s, res_lo_s);
    let res_hi = _mm256_sub_epi64(hi, carry);
    (res_hi, res_lo_s)
}

/// u64 * u32 + u64 fused multiply-add. The result is given as a tuple (u64, u64). The third
/// argument is assumed to be pre-shifted by 2^63. The result is similarly shifted.
#[inline]
unsafe fn fmadd_64_32_64s_s(x: __m256i, y: __m256i, z_s: __m256i) -> (__m256i, __m256i) {
    let x_hi = _mm256_srli_epi64(x, 32);
    let mul_lo = _mm256_mul_epu32(x, y);
    let mul_hi = _mm256_mul_epu32(x_hi, y);
    let (tmp_hi, tmp_lo_s) = add_with_carry_hi_lo_los_s(_mm256_srli_epi64(mul_hi, 32), mul_lo, z_s);
    add_with_carry_hi_lo_los_s(tmp_hi, _mm256_slli_epi64(mul_hi, 32), tmp_lo_s)
}

/// Reduce a u128 modulo FIELD_ORDER. The input is (u64, u64), pre-shifted by 2^63. The result is
/// similarly shifted.
impl ReducibleAVX2 for CrandallField {
    #[inline]
    unsafe fn reduce128s_s(x_s: (__m256i, __m256i)) -> __m256i {
        let (hi0, lo0_s) = x_s;
        let (hi1, lo1_s) = fmadd_64_32_64s_s(hi0, epsilon::<CrandallField>(), lo0_s);
        let lo2 = _mm256_mul_epu32(hi1, epsilon::<CrandallField>());
        add_no_canonicalize_64_64s_s::<CrandallField>(lo2, lo1_s)
    }
}
