use core::arch::x86_64::*;

use crate::field::crandall_field::CrandallField;
use crate::field::field_types::PrimeField;

const EPSILON: u64 = 0u64.wrapping_sub(CrandallField::ORDER);
const SIGN_BIT: u64 = 1 << 63;

const MDS_MATRIX_EXPS8: [i32; 8] = [2, 0, 1, 8, 4, 3, 0, 0];
const MDS_MATRIX_EXPS12: [i32; 12] = [10, 13, 2, 0, 4, 1, 8, 7, 15, 5, 0, 0];

#[inline(always)]
unsafe fn shift_and_accumulate<const SHIFT: i32>(
    x: __m256i,
    (hi_cumul, lo_cumul_s): (__m256i, __m256i),
) -> (__m256i, __m256i)
where
    [(); (64 - SHIFT) as usize]: ,
{
    let x_shifted_lo = _mm256_slli_epi64::<SHIFT>(x);
    let x_shifted_hi = _mm256_srli_epi64::<{ 64 - SHIFT }>(x);
    let res_lo_s = _mm256_add_epi64(lo_cumul_s, x_shifted_lo);
    let carry = _mm256_cmpgt_epi64(lo_cumul_s, res_lo_s);
    let res_hi = _mm256_sub_epi64(_mm256_add_epi64(hi_cumul, x_shifted_hi), carry);
    (res_hi, res_lo_s)
}

#[inline(always)]
unsafe fn get_vector_with_offset<const WIDTH: usize, const OFFSET: usize>(
    state: [u64; WIDTH],
) -> __m256i {
    _mm256_setr_epi64x(
        state[OFFSET % WIDTH] as i64,
        state[(OFFSET + 1) % WIDTH] as i64,
        state[(OFFSET + 2) % WIDTH] as i64,
        state[(OFFSET + 3) % WIDTH] as i64,
    )
}

#[inline(always)]
pub fn crandall_poseidon8_mds_avx2(state: [u64; 8]) -> [u64; 8] {
    unsafe {
        let mut res0_s = (_mm256_setzero_si256(), _mm256_set1_epi64x(SIGN_BIT as i64));
        let mut res1_s = (_mm256_setzero_si256(), _mm256_set1_epi64x(SIGN_BIT as i64));

        let rot0state = get_vector_with_offset::<8, 0>(state);
        let rot1state = get_vector_with_offset::<8, 1>(state);
        let rot2state = get_vector_with_offset::<8, 2>(state);
        let rot3state = get_vector_with_offset::<8, 3>(state);
        let rot4state = get_vector_with_offset::<8, 4>(state);
        let rot5state = get_vector_with_offset::<8, 5>(state);
        let rot6state = get_vector_with_offset::<8, 6>(state);
        let rot7state = get_vector_with_offset::<8, 7>(state);

        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS8[0] }>(rot0state, res0_s);
        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS8[1] }>(rot1state, res0_s);
        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS8[2] }>(rot2state, res0_s);
        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS8[3] }>(rot3state, res0_s);
        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS8[4] }>(rot4state, res0_s);
        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS8[5] }>(rot5state, res0_s);
        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS8[6] }>(rot6state, res0_s);
        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS8[7] }>(rot7state, res0_s);

        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS8[0] }>(rot4state, res1_s);
        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS8[1] }>(rot5state, res1_s);
        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS8[2] }>(rot6state, res1_s);
        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS8[3] }>(rot7state, res1_s);
        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS8[4] }>(rot0state, res1_s);
        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS8[5] }>(rot1state, res1_s);
        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS8[6] }>(rot2state, res1_s);
        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS8[7] }>(rot3state, res1_s);

        let reduced0 = reduce96s(res0_s);
        let reduced1 = reduce96s(res1_s);
        [
            _mm256_extract_epi64::<0>(reduced0) as u64,
            _mm256_extract_epi64::<1>(reduced0) as u64,
            _mm256_extract_epi64::<2>(reduced0) as u64,
            _mm256_extract_epi64::<3>(reduced0) as u64,
            _mm256_extract_epi64::<0>(reduced1) as u64,
            _mm256_extract_epi64::<1>(reduced1) as u64,
            _mm256_extract_epi64::<2>(reduced1) as u64,
            _mm256_extract_epi64::<3>(reduced1) as u64,
        ]
    }
}

#[inline(always)]
pub fn crandall_poseidon12_mds_avx2(state: [u64; 12]) -> [u64; 12] {
    unsafe {
        let mut res0_s = (_mm256_setzero_si256(), _mm256_set1_epi64x(SIGN_BIT as i64));
        let mut res1_s = (_mm256_setzero_si256(), _mm256_set1_epi64x(SIGN_BIT as i64));
        let mut res2_s = (_mm256_setzero_si256(), _mm256_set1_epi64x(SIGN_BIT as i64));

        let rot0state = get_vector_with_offset::<12, 0>(state);
        let rot1state = get_vector_with_offset::<12, 1>(state);
        let rot2state = get_vector_with_offset::<12, 2>(state);
        let rot3state = get_vector_with_offset::<12, 3>(state);
        let rot4state = get_vector_with_offset::<12, 4>(state);
        let rot5state = get_vector_with_offset::<12, 5>(state);
        let rot6state = get_vector_with_offset::<12, 6>(state);
        let rot7state = get_vector_with_offset::<12, 7>(state);
        let rot8state = get_vector_with_offset::<12, 8>(state);
        let rot9state = get_vector_with_offset::<12, 9>(state);
        let rot10state = get_vector_with_offset::<12, 10>(state);
        let rot11state = get_vector_with_offset::<12, 11>(state);

        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[0] }>(rot0state, res0_s);
        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[1] }>(rot1state, res0_s);
        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[2] }>(rot2state, res0_s);
        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[3] }>(rot3state, res0_s);
        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[4] }>(rot4state, res0_s);
        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[5] }>(rot5state, res0_s);
        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[6] }>(rot6state, res0_s);
        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[7] }>(rot7state, res0_s);
        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[8] }>(rot8state, res0_s);
        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[9] }>(rot9state, res0_s);
        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[10] }>(rot10state, res0_s);
        res0_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[11] }>(rot11state, res0_s);

        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[0] }>(rot4state, res1_s);
        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[1] }>(rot5state, res1_s);
        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[2] }>(rot6state, res1_s);
        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[3] }>(rot7state, res1_s);
        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[4] }>(rot8state, res1_s);
        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[5] }>(rot9state, res1_s);
        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[6] }>(rot10state, res1_s);
        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[7] }>(rot11state, res1_s);
        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[8] }>(rot0state, res1_s);
        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[9] }>(rot1state, res1_s);
        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[10] }>(rot2state, res1_s);
        res1_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[11] }>(rot3state, res1_s);

        res2_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[0] }>(rot8state, res2_s);
        res2_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[1] }>(rot9state, res2_s);
        res2_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[2] }>(rot10state, res2_s);
        res2_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[3] }>(rot11state, res2_s);
        res2_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[4] }>(rot0state, res2_s);
        res2_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[5] }>(rot1state, res2_s);
        res2_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[6] }>(rot2state, res2_s);
        res2_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[7] }>(rot3state, res2_s);
        res2_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[8] }>(rot4state, res2_s);
        res2_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[9] }>(rot5state, res2_s);
        res2_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[10] }>(rot6state, res2_s);
        res2_s = shift_and_accumulate::<{ MDS_MATRIX_EXPS12[11] }>(rot7state, res2_s);

        let reduced0 = reduce96s(res0_s);
        let reduced1 = reduce96s(res1_s);
        let reduced2 = reduce96s(res2_s);
        [
            _mm256_extract_epi64::<0>(reduced0) as u64,
            _mm256_extract_epi64::<1>(reduced0) as u64,
            _mm256_extract_epi64::<2>(reduced0) as u64,
            _mm256_extract_epi64::<3>(reduced0) as u64,
            _mm256_extract_epi64::<0>(reduced1) as u64,
            _mm256_extract_epi64::<1>(reduced1) as u64,
            _mm256_extract_epi64::<2>(reduced1) as u64,
            _mm256_extract_epi64::<3>(reduced1) as u64,
            _mm256_extract_epi64::<0>(reduced2) as u64,
            _mm256_extract_epi64::<1>(reduced2) as u64,
            _mm256_extract_epi64::<2>(reduced2) as u64,
            _mm256_extract_epi64::<3>(reduced2) as u64,
        ]
    }
}

#[inline(always)]
unsafe fn reduce96s(x_s: (__m256i, __m256i)) -> __m256i {
    let (hi0, lo0_s) = x_s;
    let lo1 = _mm256_mul_epu32(hi0, _mm256_set1_epi64x(EPSILON as i64));
    add_no_canonicalize_64_64s(lo1, lo0_s)
}

#[inline(always)]
unsafe fn add_no_canonicalize_64_64s(x: __m256i, y_s: __m256i) -> __m256i {
    let res_wrapped_s = _mm256_add_epi64(x, y_s);
    let mask = _mm256_cmpgt_epi64(y_s, res_wrapped_s);
    let res_wrapped = _mm256_xor_epi64(res_wrapped_s, _mm256_set1_epi64x(SIGN_BIT as i64));
    let wrapback_amt = _mm256_and_si256(mask, _mm256_set1_epi64x(EPSILON as i64));
    let res = _mm256_add_epi64(res_wrapped, wrapback_amt);
    res
}
