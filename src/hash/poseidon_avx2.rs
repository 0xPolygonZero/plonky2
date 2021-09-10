use core::arch::x86_64::*;

const EPSILON: u64 = (1 << 31) + (1 << 28) - 1;
const SIGN_BIT: u64 = 1 << 63;

#[inline(always)]
unsafe fn shift_and_accumulate<const SHIFT: i32>(
    x: __m256i,
    (hi_cumul, lo_cumul_s): (__m256i, __m256i),
) -> (__m256i, __m256i)
where
    [(); (64 - SHIFT) as usize]: ,
{
    let x_shifted_lo = _mm256_slli_epi64(x, SHIFT);
    let x_shifted_hi = _mm256_srli_epi64(x, 64 - SHIFT);
    let res_lo_s = _mm256_add_epi64(lo_cumul_s, x_shifted_lo);
    let carry = _mm256_cmpgt_epi64(lo_cumul_s, res_lo_s);
    let res_hi = _mm256_sub_epi64(_mm256_add_epi64(hi_cumul, x_shifted_hi), carry);
    (res_hi, res_lo_s)
}

#[inline(always)]
pub fn crandall_poseidon8_mds_avx2(state: [u64; 8]) -> [u64; 8] {
    unsafe {
        let mut res0_s = (_mm256_setzero_si256(), _mm256_set1_epi64x(SIGN_BIT as i64));
        let mut res1_s = (_mm256_setzero_si256(), _mm256_set1_epi64x(SIGN_BIT as i64));

        let state_extended: [u64; 12] = [
            state[0], state[1], state[2], state[3], state[4], state[5], state[6], state[7],
            state[0], state[1], state[2], state[3],
        ];

        let rot0state = _mm256_loadu_si256(state_extended[0..4].as_ptr().cast::<__m256i>());
        let rot1state = _mm256_loadu_si256(state_extended[1..5].as_ptr().cast::<__m256i>());
        let rot2state = _mm256_loadu_si256(state_extended[2..6].as_ptr().cast::<__m256i>());
        let rot3state = _mm256_loadu_si256(state_extended[3..7].as_ptr().cast::<__m256i>());
        let rot4state = _mm256_loadu_si256(state_extended[4..8].as_ptr().cast::<__m256i>());
        let rot5state = _mm256_loadu_si256(state_extended[5..9].as_ptr().cast::<__m256i>());
        let rot6state = _mm256_loadu_si256(state_extended[6..10].as_ptr().cast::<__m256i>());
        let rot7state = _mm256_loadu_si256(state_extended[7..11].as_ptr().cast::<__m256i>());

        res0_s = shift_and_accumulate::<2>(rot0state, res0_s);
        res0_s = shift_and_accumulate::<0>(rot1state, res0_s);
        res0_s = shift_and_accumulate::<1>(rot2state, res0_s);
        res0_s = shift_and_accumulate::<8>(rot3state, res0_s);
        res0_s = shift_and_accumulate::<4>(rot4state, res0_s);
        res0_s = shift_and_accumulate::<3>(rot5state, res0_s);
        res0_s = shift_and_accumulate::<0>(rot6state, res0_s);
        res0_s = shift_and_accumulate::<0>(rot7state, res0_s);

        res1_s = shift_and_accumulate::<2>(rot4state, res1_s);
        res1_s = shift_and_accumulate::<0>(rot5state, res1_s);
        res1_s = shift_and_accumulate::<1>(rot6state, res1_s);
        res1_s = shift_and_accumulate::<8>(rot7state, res1_s);
        res1_s = shift_and_accumulate::<4>(rot0state, res1_s);
        res1_s = shift_and_accumulate::<3>(rot1state, res1_s);
        res1_s = shift_and_accumulate::<0>(rot2state, res1_s);
        res1_s = shift_and_accumulate::<0>(rot3state, res1_s);

        // Finalize
        let reduced0 = reduce96s(res0_s);
        let reduced1 = reduce96s(res1_s);

        let mut reduced = [0u64; 8];
        _mm256_storeu_si256(reduced[0..4].as_mut_ptr().cast::<__m256i>(), reduced0);
        _mm256_storeu_si256(reduced[4..8].as_mut_ptr().cast::<__m256i>(), reduced1);
        reduced
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
