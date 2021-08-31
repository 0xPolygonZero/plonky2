use core::arch::x86_64::*;

const EPSILON: u64 = (1 << 31) + (1 << 28) - 1;

pub struct CrandallFieldVec512();

#[inline]
unsafe fn epsilon() -> __m512i {
    _mm512_set1_epi64(EPSILON as i64)
}

#[inline]
unsafe fn one() -> __m512i {
    _mm512_set1_epi64(1)
}

#[inline]
unsafe fn mul64_64(x: __m512i, y: __m512i) -> (__m512i, __m512i) {
    let x_hi = _mm512_srli_epi64(x, 32);
    let y_hi = _mm512_srli_epi64(y, 32);
    let mul_ll = _mm512_mul_epu32(x, y);
    let mul_lh = _mm512_mul_epu32(x, y_hi);
    let mul_hl = _mm512_mul_epu32(x_hi, y);
    let mul_hh = _mm512_mul_epu32(x_hi, y_hi);

    let res_lo0 = mul_ll;
    let res_hi0 = mul_hh;

    let res_lo1 = _mm512_add_epi32(res_lo0, _mm512_slli_epi64(mul_lh, 32));
    let carry1 = _mm512_cmpgt_epu64_mask(res_lo0, res_lo1);
    let res_hi1 = _mm512_mask_add_epi64(res_hi0, carry1, res_hi0, one());

    let res_lo2 = _mm512_add_epi32(res_lo1, _mm512_slli_epi64(mul_hl, 32));
    let carry2 = _mm512_cmpgt_epu64_mask(res_lo1, res_lo2);
    let res_hi2 = _mm512_mask_add_epi64(res_hi1, carry2, res_hi1, one());

    let res_hi3 = _mm512_add_epi64(res_hi2, _mm512_srli_epi64(mul_lh, 32));
    let res_hi4 = _mm512_add_epi64(res_hi3, _mm512_srli_epi64(mul_hl, 32));

    (res_hi4, res_lo2)
}

#[inline]
unsafe fn add_with_carry128_64(x: (__m512i, __m512i), y: __m512i) -> (__m512i, __m512i) {
    let (x_hi, x_lo) = x;
    let res_lo = _mm512_add_epi64(x_lo, y);
    let carry = _mm512_cmpgt_epu64_mask(x_lo, res_lo);
    let res_hi = _mm512_mask_add_epi64(x_hi, carry, x_hi, one());
    (res_hi, res_lo)
}

#[inline]
unsafe fn fmadd_64_32_64(x: __m512i, y: __m512i, z: __m512i) -> (__m512i, __m512i) {
    let x_hi = _mm512_srli_epi64(x, 32);
    let mul_lo = _mm512_mul_epu32(x, y);
    let mul_hi = _mm512_mul_epu32(x_hi, y);
    let tmp = add_with_carry128_64(
        (_mm512_srli_epi64(mul_hi, 32), _mm512_slli_epi64(mul_hi, 32)),
        z,
    );
    add_with_carry128_64(tmp, mul_lo)
}

#[inline]
unsafe fn reduce128(x: (__m512i, __m512i)) -> __m512i {
    let (hi0, lo0) = x;
    let (hi1, lo1) = fmadd_64_32_64(hi0, epsilon(), lo0);
    let lo2 = _mm512_mul_epu32(hi1, epsilon());
    let res_wrapped = _mm512_add_epi64(lo1, lo2);
    let carry = _mm512_cmpgt_epu64_mask(lo1, res_wrapped);
    let res = _mm512_mask_add_epi64(res_wrapped, carry, res_wrapped, epsilon());
    res
}

impl CrandallFieldVec512 {
    #[inline]
    pub unsafe fn mul(x: __m512i, y: __m512i) -> __m512i {
        reduce128(mul64_64(x, y))
    }
}
