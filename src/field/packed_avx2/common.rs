use core::arch::x86_64::*;

use crate::field::field_types::PrimeField;

pub trait ReducibleAVX2: PrimeField {
    unsafe fn reduce128(x: (__m256i, __m256i)) -> __m256i;
}

const SIGN_BIT: u64 = 1 << 63;

#[inline]
unsafe fn sign_bit() -> __m256i {
    _mm256_set1_epi64x(SIGN_BIT as i64)
}

/// Add 2^63 with overflow. Needed to emulate unsigned comparisons (see point 3. in
/// packed_prime_field.rs).
#[inline]
pub unsafe fn shift(x: __m256i) -> __m256i {
    _mm256_xor_si256(x, sign_bit())
}

#[inline]
pub unsafe fn field_order<F: PrimeField>() -> __m256i {
    _mm256_set1_epi64x(F::ORDER as i64)
}

#[inline]
pub unsafe fn epsilon<F: PrimeField>() -> __m256i {
    _mm256_set1_epi64x(0u64.wrapping_sub(F::ORDER) as i64)
}

/// Addition u64 + u64 -> u64. Assumes that x + y < 2^64 + FIELD_ORDER. The second argument is
/// pre-shifted by 1 << 63. The result is similarly shifted.
#[inline]
pub unsafe fn add_no_canonicalize_64_64s_s<F: PrimeField>(x: __m256i, y_s: __m256i) -> __m256i {
    let res_wrapped_s = _mm256_add_epi64(x, y_s);
    let mask = _mm256_cmpgt_epi64(y_s, res_wrapped_s); // -1 if overflowed else 0.
    let wrapback_amt = _mm256_and_si256(mask, epsilon::<F>()); // -FIELD_ORDER if overflowed else 0.
    let res_s = _mm256_add_epi64(res_wrapped_s, wrapback_amt);
    res_s
}

/// Subtraction u64 - u64 -> u64. Assumes that double overflow cannot occur. The first argument is
/// pre-shifted by 1 << 63 and the result is similarly shifted.
#[inline]
pub unsafe fn sub_no_canonicalize_64s_64_s<F: PrimeField>(x_s: __m256i, y: __m256i) -> __m256i {
    let res_wrapped_s = _mm256_sub_epi64(x_s, y);
    let mask = _mm256_cmpgt_epi64(res_wrapped_s, x_s); // -1 if overflowed else 0.
    let wrapback_amt = _mm256_and_si256(mask, epsilon::<F>()); // -FIELD_ORDER if overflowed else 0.
    let res_s = _mm256_sub_epi64(res_wrapped_s, wrapback_amt);
    res_s
}
