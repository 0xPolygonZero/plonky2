use core::arch::x86_64::*;

use crate::field::goldilocks_field::GoldilocksField;
use crate::field::packed_avx2::common::{
    add_no_canonicalize_64_64s_s, epsilon, shift, sub_no_canonicalize_64s_64_s, ReducibleAVX2,
};

/// Reduce a u128 modulo FIELD_ORDER. The input is (u64, u64), pre-shifted by 2^63. The result is
/// similarly shifted.
impl ReducibleAVX2 for GoldilocksField {
    #[inline]
    unsafe fn reduce128(x: (__m256i, __m256i)) -> __m256i {
        let (hi0, lo0) = x;
        let lo0_s = shift(lo0);
        let hi_hi0 = _mm256_srli_epi64(hi0, 32);
        let lo1_s = sub_no_canonicalize_64s_64_s::<GoldilocksField>(lo0_s, hi_hi0);
        let t1 = _mm256_mul_epu32(hi0, epsilon::<GoldilocksField>());
        let lo2_s = add_no_canonicalize_64_64s_s::<GoldilocksField>(t1, lo1_s);
        let lo2 = shift(lo2_s);
        lo2
    }
}
