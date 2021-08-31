use core::arch::x86_64::*;

const EPSILON: u64 = (1 << 31) + (1 << 28) - 1;
const SIGN_BIT: u64 = 1 << 63;

pub struct CrandallFieldVec();

#[inline]
unsafe fn epsilon() -> __m256i {
    _mm256_set1_epi64x(EPSILON as i64)
}

#[inline]
unsafe fn sign_bit() -> __m256i {
    _mm256_set1_epi64x(SIGN_BIT as i64)
}

#[inline]
unsafe fn shift(x: __m256i) -> __m256i {
    _mm256_xor_si256(x, sign_bit())
}

#[inline]
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

#[inline]
unsafe fn add_with_carry128_64s_s(x: (__m256i, __m256i), y_s: __m256i) -> (__m256i, __m256i) {
    let (x_hi, x_lo) = x;
    let res_lo_s = _mm256_add_epi64(x_lo, y_s);
    let carry = _mm256_cmpgt_epi64(y_s, res_lo_s);
    let res_hi = _mm256_sub_epi64(x_hi, carry);
    (res_hi, res_lo_s)
}

#[inline]
unsafe fn add_with_carry128s_64_s(x_s: (__m256i, __m256i), y: __m256i) -> (__m256i, __m256i) {
    let (x_hi, x_lo_s) = x_s;
    let res_lo_s = _mm256_add_epi64(x_lo_s, y);
    let carry = _mm256_cmpgt_epi64(x_lo_s, res_lo_s);
    let res_hi = _mm256_sub_epi64(x_hi, carry);
    (res_hi, res_lo_s)
}

#[inline]
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

#[inline]
unsafe fn reduce128s_s(x_s: (__m256i, __m256i)) -> __m256i {
    let (hi0, lo0_s) = x_s;
    let (hi1, lo1_s) = fmadd_64_32_64s_s(hi0, epsilon(), lo0_s);
    let lo2 = _mm256_mul_epu32(hi1, epsilon());
    let res_wrapped_s = _mm256_add_epi64(lo1_s, lo2);
    let carry_mask = _mm256_cmpgt_epi64(lo1_s, res_wrapped_s); // all 1 if overflow
    let res_s = _mm256_add_epi64(res_wrapped_s, _mm256_and_si256(carry_mask, epsilon()));
    res_s
}

impl CrandallFieldVec {
    #[inline]
    pub unsafe fn mul(x: __m256i, y: __m256i) -> __m256i {
        shift(reduce128s_s(mul64_64_s(x, y)))
    }
}
