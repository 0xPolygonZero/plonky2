use core::arch::x86_64::*;

const EPSILON: u64 = (1 << 32) - 1;
const SIGN_BIT: u64 = 1 << 63;

pub struct GoldilocksFieldVec();

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
unsafe fn sub_modulo_64s_64_s(x_s: __m256i, y: __m256i) -> __m256i {
    let t0_s = _mm256_sub_epi64(x_s, y);
    let carry_mask = _mm256_cmpgt_epi64(t0_s, x_s);
    let adj = _mm256_and_si256(carry_mask, epsilon());
    let t1_s = _mm256_sub_epi64(t0_s, adj);
    t1_s
}

#[inline]
unsafe fn add_modulo_64s_64_s(x_s: __m256i, y: __m256i) -> __m256i {
    let t0_s = _mm256_add_epi64(x_s, y);
    let carry_mask = _mm256_cmpgt_epi64(x_s, t0_s);
    let adj = _mm256_and_si256(carry_mask, epsilon());
    let t1_s = _mm256_add_epi64(t0_s, adj);
    t1_s
}

#[inline]
unsafe fn reduce128s_s(x_s: (__m256i, __m256i)) -> __m256i {
    let (hi0, lo0_s) = x_s;
    let hi_hi0 = _mm256_srli_epi64(hi0, 32);
    let lo1_s = sub_modulo_64s_64_s(lo0_s, hi_hi0);
    let t1 = _mm256_mul_epu32(hi0, epsilon());
    let lo2_s = add_modulo_64s_64_s(lo1_s, t1);;
    lo2_s
}

impl GoldilocksFieldVec {
    #[inline]
    pub unsafe fn mul(x: __m256i, y: __m256i) -> __m256i {
        shift(reduce128s_s(mul64_64_s(x, y)))
    }
}
