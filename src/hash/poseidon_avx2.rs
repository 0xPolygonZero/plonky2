use core::arch::x86_64::*;

const EPSILON: u64 = (1 << 31) + (1 << 28) - 1;
const SIGN_BIT: u64 = 1 << 63;

#[inline(always)]
pub fn crandall_poseidon8_mds_avx2(state: [u64; 8]) -> [u64; 8] {
    unsafe {
        let res0lo0_s = _mm256_set1_epi64x(SIGN_BIT as i64);
        let res0lo1_s = _mm256_set1_epi64x(SIGN_BIT as i64);
        let res0hi0 = _mm256_setzero_si256();
        let res0hi1 = _mm256_setzero_si256();

        let state_extended: [u64; 12] = [
            state[0], state[1], state[2], state[3], state[4], state[5], state[6], state[7],
            state[0], state[1], state[2], state[3],
        ];

        let rot0state0 = _mm256_loadu_si256(state_extended[0..4].as_ptr().cast::<__m256i>());
        let rot1state0 = _mm256_loadu_si256(state_extended[1..5].as_ptr().cast::<__m256i>());
        let rot2state0 = _mm256_loadu_si256(state_extended[2..6].as_ptr().cast::<__m256i>());
        let rot3state0 = _mm256_loadu_si256(state_extended[3..7].as_ptr().cast::<__m256i>());
        let rot4state0 = _mm256_loadu_si256(state_extended[4..8].as_ptr().cast::<__m256i>());
        let rot5state0 = _mm256_loadu_si256(state_extended[5..9].as_ptr().cast::<__m256i>());
        let rot6state0 = _mm256_loadu_si256(state_extended[6..10].as_ptr().cast::<__m256i>());
        let rot7state0 = _mm256_loadu_si256(state_extended[7..11].as_ptr().cast::<__m256i>());

        let rot0state1 = rot4state0;
        let rot1state1 = rot5state0;
        let rot2state1 = rot6state0;
        let rot3state1 = rot7state0;
        let rot4state1 = rot0state0;
        let rot5state1 = rot1state0;
        let rot6state1 = rot2state0;
        let rot7state1 = rot3state0;

        // Perform shifts
        let rot0lo0 = _mm256_slli_epi64(rot0state0, 2);
        let rot0lo1 = _mm256_slli_epi64(rot0state1, 2);
        let rot1lo0 = rot1state0;
        let rot1lo1 = rot1state1;
        let rot2lo0 = _mm256_add_epi64(rot2state0, rot2state0);
        let rot2lo1 = _mm256_add_epi64(rot2state1, rot2state1);
        let rot3lo0 = _mm256_slli_epi64(rot3state0, 8);
        let rot3lo1 = _mm256_slli_epi64(rot3state1, 8);
        let rot4lo0 = _mm256_slli_epi64(rot4state0, 4);
        let rot4lo1 = _mm256_slli_epi64(rot4state1, 4);
        let rot5lo0 = _mm256_slli_epi64(rot5state0, 3);
        let rot5lo1 = _mm256_slli_epi64(rot5state1, 3);
        let rot6lo0 = rot6state0;
        let rot6lo1 = rot6state1;
        let rot7lo0 = rot7state0;
        let rot7lo1 = rot7state1;

        let rot0hi0 = _mm256_srli_epi64(rot0state0, 62);
        let rot0hi1 = _mm256_srli_epi64(rot0state1, 62);
        // rot1hi0 is 0
        // rot1hi1 is 0
        let rot2hi0 = _mm256_srli_epi64(rot2state0, 63);
        let rot2hi1 = _mm256_srli_epi64(rot2state1, 63);
        let rot3hi0 = _mm256_srli_epi64(rot3state0, 56);
        let rot3hi1 = _mm256_srli_epi64(rot3state1, 56);
        let rot4hi0 = _mm256_srli_epi64(rot4state0, 60);
        let rot4hi1 = _mm256_srli_epi64(rot4state1, 60);
        let rot5hi0 = _mm256_srli_epi64(rot5state0, 61);
        let rot5hi1 = _mm256_srli_epi64(rot5state1, 61);
        // rot6hi0 is 0
        // rot6hi1 is 0
        // rot7hi0 is 0
        // rot7hi1 is 0

        // Additions
        let res1lo0_s = _mm256_add_epi64(res0lo0_s, rot0lo0);
        let res1lo1_s = _mm256_add_epi64(res0lo1_s, rot0lo1);
        let res2lo0_s = _mm256_add_epi64(res1lo0_s, rot1lo0);
        let res2lo1_s = _mm256_add_epi64(res1lo1_s, rot1lo1);
        let res3lo0_s = _mm256_add_epi64(res2lo0_s, rot2lo0);
        let res3lo1_s = _mm256_add_epi64(res2lo1_s, rot2lo1);
        let res4lo0_s = _mm256_add_epi64(res3lo0_s, rot3lo0);
        let res4lo1_s = _mm256_add_epi64(res3lo1_s, rot3lo1);
        let res5lo0_s = _mm256_add_epi64(res4lo0_s, rot4lo0);
        let res5lo1_s = _mm256_add_epi64(res4lo1_s, rot4lo1);
        let res6lo0_s = _mm256_add_epi64(res5lo0_s, rot5lo0);
        let res6lo1_s = _mm256_add_epi64(res5lo1_s, rot5lo1);
        let res7lo0_s = _mm256_add_epi64(res6lo0_s, rot6lo0);
        let res7lo1_s = _mm256_add_epi64(res6lo1_s, rot6lo1);
        let res8lo0_s = _mm256_add_epi64(res7lo0_s, rot7lo0);
        let res8lo1_s = _mm256_add_epi64(res7lo1_s, rot7lo1);

        let res1hi0 = _mm256_add_epi64(res0hi0, rot0hi0);
        let res1hi1 = _mm256_add_epi64(res0hi1, rot0hi1);
        let res2hi0 = res1hi0;
        let res2hi1 = res1hi1;
        let res3hi0 = _mm256_add_epi64(res2hi0, rot2hi0);
        let res3hi1 = _mm256_add_epi64(res2hi1, rot2hi1);
        let res4hi0 = _mm256_add_epi64(res3hi0, rot3hi0);
        let res4hi1 = _mm256_add_epi64(res3hi1, rot3hi1);
        let res5hi0 = _mm256_add_epi64(res4hi0, rot4hi0);
        let res5hi1 = _mm256_add_epi64(res4hi1, rot4hi1);
        let res6hi0 = _mm256_add_epi64(res5hi0, rot5hi0);
        let res6hi1 = _mm256_add_epi64(res5hi1, rot5hi1);
        let res7hi0 = res6hi0;
        let res7hi1 = res6hi1;
        let res8hi0 = res7hi0;
        let res8hi1 = res7hi1;

        // Carries
        let res1carry0 = _mm256_cmpgt_epi64(res0lo0_s, res1lo0_s);
        let res1carry1 = _mm256_cmpgt_epi64(res0lo1_s, res1lo1_s);
        let res2carry0 = _mm256_cmpgt_epi64(res1lo0_s, res2lo0_s);
        let res2carry1 = _mm256_cmpgt_epi64(res1lo1_s, res2lo1_s);
        let res3carry0 = _mm256_cmpgt_epi64(res2lo0_s, res3lo0_s);
        let res3carry1 = _mm256_cmpgt_epi64(res2lo1_s, res3lo1_s);
        let res4carry0 = _mm256_cmpgt_epi64(res3lo0_s, res4lo0_s);
        let res4carry1 = _mm256_cmpgt_epi64(res3lo1_s, res4lo1_s);
        let res5carry0 = _mm256_cmpgt_epi64(res4lo0_s, res5lo0_s);
        let res5carry1 = _mm256_cmpgt_epi64(res4lo1_s, res5lo1_s);
        let res6carry0 = _mm256_cmpgt_epi64(res5lo0_s, res6lo0_s);
        let res6carry1 = _mm256_cmpgt_epi64(res5lo1_s, res6lo1_s);
        let res7carry0 = _mm256_cmpgt_epi64(res6lo0_s, res7lo0_s);
        let res7carry1 = _mm256_cmpgt_epi64(res6lo1_s, res7lo1_s);
        let res8carry0 = _mm256_cmpgt_epi64(res7lo0_s, res8lo0_s);
        let res8carry1 = _mm256_cmpgt_epi64(res7lo1_s, res8lo1_s);

        let res9hi0 = _mm256_sub_epi64(res8hi0, res1carry0);
        let res9hi1 = _mm256_sub_epi64(res8hi1, res1carry1);
        let res10hi0 = _mm256_sub_epi64(res9hi0, res2carry0);
        let res10hi1 = _mm256_sub_epi64(res9hi1, res2carry1);
        let res11hi0 = _mm256_sub_epi64(res10hi0, res3carry0);
        let res11hi1 = _mm256_sub_epi64(res10hi1, res3carry1);
        let res12hi0 = _mm256_sub_epi64(res11hi0, res4carry0);
        let res12hi1 = _mm256_sub_epi64(res11hi1, res4carry1);
        let res13hi0 = _mm256_sub_epi64(res12hi0, res5carry0);
        let res13hi1 = _mm256_sub_epi64(res12hi1, res5carry1);
        let res14hi0 = _mm256_sub_epi64(res13hi0, res6carry0);
        let res14hi1 = _mm256_sub_epi64(res13hi1, res6carry1);
        let res15hi0 = _mm256_sub_epi64(res14hi0, res7carry0);
        let res15hi1 = _mm256_sub_epi64(res14hi1, res7carry1);
        let res16hi0 = _mm256_sub_epi64(res15hi0, res8carry0);
        let res16hi1 = _mm256_sub_epi64(res15hi1, res8carry1);

        // Finalize
        let reduced0 = reduce96s((res16hi0, res8lo0_s));
        let reduced1 = reduce96s((res16hi1, res8lo1_s));

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
