use core::arch::x86_64::*;

use unroll::unroll_for_loops;

use crate::field::crandall_field::CrandallField;
use crate::field::field_types::{Field, PrimeField};
use crate::field::packed_avx2::PackedCrandallAVX2;
use crate::field::packed_field::PackedField;

const EPSILON: u64 = 0u64.wrapping_sub(CrandallField::ORDER);
const SIGN_BIT: u64 = 1 << 63;

const MDS_MATRIX_EXPS8: [i32; 8] = [2, 0, 1, 8, 4, 3, 0, 0];
const MDS_MATRIX_EXPS12: [i32; 12] = [10, 13, 2, 0, 4, 1, 8, 7, 15, 5, 0, 0];

/// Pair of vectors (hi, lo) representing a u128.
type Vecs128 = (__m256i, __m256i);

/// Takes cumul (u128) and x (u64). Returns cumul + (x << SHIFT) as u128.
/// Assumes that cumul is shifted by 1 << 63; the result is similarly shifted.
#[inline(always)]
unsafe fn shift_and_accumulate<const SHIFT: i32>(
    x: __m256i,
    (hi_cumul, lo_cumul_s): Vecs128,
) -> Vecs128
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

/// Extract state[OFFSET..OFFSET + 4] as a vector. Wraps around the boundary.
#[inline(always)]
unsafe fn get_vector_with_offset<const WIDTH: usize, const OFFSET: usize>(
    state: [CrandallField; WIDTH],
) -> __m256i {
    _mm256_setr_epi64x(
        state[OFFSET % WIDTH].0 as i64,
        state[(OFFSET + 1) % WIDTH].0 as i64,
        state[(OFFSET + 2) % WIDTH].0 as i64,
        state[(OFFSET + 3) % WIDTH].0 as i64,
    )
}

/// Extract CrandallField element from vector.
#[inline(always)]
unsafe fn extract<const INDEX: i32>(v: __m256i) -> CrandallField {
    CrandallField(_mm256_extract_epi64::<INDEX>(v) as u64)
}

#[inline(always)]
unsafe fn iteration8<const INDEX: usize, const SHIFT: i32>(
    [cumul0_s, cumul1_s]: [Vecs128; 2],
    state: [CrandallField; 8],
) -> [Vecs128; 2]
// 2 vectors of 4 needed to represent entire state.
where
    [(); INDEX + 4]: ,
    [(); (64 - SHIFT) as usize]: ,
{
    // Entire state, rotated by INDEX.
    let state0 = get_vector_with_offset::<8, INDEX>(state);
    let state1 = get_vector_with_offset::<8, { INDEX + 4 }>(state);
    [
        shift_and_accumulate::<SHIFT>(state0, cumul0_s),
        shift_and_accumulate::<SHIFT>(state1, cumul1_s),
    ]
}

#[inline(always)]
pub fn crandall_poseidon8_mds_avx2(state: [CrandallField; 8]) -> [CrandallField; 8] {
    unsafe {
        let mut res_s = [(_mm256_setzero_si256(), _mm256_set1_epi64x(SIGN_BIT as i64)); 2];

        // The scalar loop goes:
        //     for r in 0..WIDTH {
        //         let mut res = 0u128;
        //         for i in 0..WIDTH {
        //             res += (state[(i + r) % WIDTH] as u128) << MDS_MATRIX_EXPS[i];
        //         }
        //         result[r] = reduce(res);
        //     }
        //
        // Here, we swap the loops. Equivalent to:
        //     let mut res = [0u128; WIDTH];
        //     for i in 0..WIDTH {
        //         let mds_matrix_exp = MDS_MATRIX_EXPS[i];
        //         for r in 0..WIDTH {
        //             res[r] += (state[(i + r) % WIDTH] as u128) << mds_matrix_exp;
        //         }
        //     }
        //     for r in 0..WIDTH {
        //         result[r] = reduce(res[r]);
        //     }
        //
        // Notice that that in the lower version, all iterations of the inner loop shift by the same
        // amount. In vector, we perform multiple iterations of the loop at once, and vector shifts
        // are cheaper when all elements are shifted by the same amount.

        res_s = iteration8::<0, { MDS_MATRIX_EXPS8[0] }>(res_s, state);
        res_s = iteration8::<1, { MDS_MATRIX_EXPS8[1] }>(res_s, state);
        res_s = iteration8::<2, { MDS_MATRIX_EXPS8[2] }>(res_s, state);
        res_s = iteration8::<3, { MDS_MATRIX_EXPS8[3] }>(res_s, state);
        res_s = iteration8::<4, { MDS_MATRIX_EXPS8[4] }>(res_s, state);
        res_s = iteration8::<5, { MDS_MATRIX_EXPS8[5] }>(res_s, state);
        res_s = iteration8::<6, { MDS_MATRIX_EXPS8[6] }>(res_s, state);
        res_s = iteration8::<7, { MDS_MATRIX_EXPS8[7] }>(res_s, state);

        let [res0_s, res1_s] = res_s;
        let reduced0 = reduce96s(res0_s);
        let reduced1 = reduce96s(res1_s);
        [
            extract::<0>(reduced0),
            extract::<1>(reduced0),
            extract::<2>(reduced0),
            extract::<3>(reduced0),
            extract::<0>(reduced1),
            extract::<1>(reduced1),
            extract::<2>(reduced1),
            extract::<3>(reduced1),
        ]
    }
}

#[inline(always)]
unsafe fn iteration12<const INDEX: usize, const SHIFT: i32>(
    [cumul0_s, cumul1_s, cumul2_s]: [Vecs128; 3],
    state: [CrandallField; 12],
) -> [Vecs128; 3]
// 3 vectors of 4 needed to represent entire state.
where
    [(); INDEX + 4]: ,
    [(); INDEX + 8]: ,
    [(); (64 - SHIFT) as usize]: ,
{
    // Entire state, rotated by INDEX.
    let state0 = get_vector_with_offset::<12, INDEX>(state);
    let state1 = get_vector_with_offset::<12, { INDEX + 4 }>(state);
    let state2 = get_vector_with_offset::<12, { INDEX + 8 }>(state);
    [
        shift_and_accumulate::<SHIFT>(state0, cumul0_s),
        shift_and_accumulate::<SHIFT>(state1, cumul1_s),
        shift_and_accumulate::<SHIFT>(state2, cumul2_s),
    ]
}

#[inline(always)]
pub fn crandall_poseidon12_mds_avx2(state: [CrandallField; 12]) -> [CrandallField; 12] {
    unsafe {
        let mut res_s = [(_mm256_setzero_si256(), _mm256_set1_epi64x(SIGN_BIT as i64)); 3];

        // See width-8 version for explanation.

        res_s = iteration12::<0, { MDS_MATRIX_EXPS12[0] }>(res_s, state);
        res_s = iteration12::<1, { MDS_MATRIX_EXPS12[1] }>(res_s, state);
        res_s = iteration12::<2, { MDS_MATRIX_EXPS12[2] }>(res_s, state);
        res_s = iteration12::<3, { MDS_MATRIX_EXPS12[3] }>(res_s, state);
        res_s = iteration12::<4, { MDS_MATRIX_EXPS12[4] }>(res_s, state);
        res_s = iteration12::<5, { MDS_MATRIX_EXPS12[5] }>(res_s, state);
        res_s = iteration12::<6, { MDS_MATRIX_EXPS12[6] }>(res_s, state);
        res_s = iteration12::<7, { MDS_MATRIX_EXPS12[7] }>(res_s, state);
        res_s = iteration12::<8, { MDS_MATRIX_EXPS12[8] }>(res_s, state);
        res_s = iteration12::<9, { MDS_MATRIX_EXPS12[9] }>(res_s, state);
        res_s = iteration12::<10, { MDS_MATRIX_EXPS12[10] }>(res_s, state);
        res_s = iteration12::<11, { MDS_MATRIX_EXPS12[11] }>(res_s, state);

        let [res0_s, res1_s, res2_s] = res_s;
        let reduced0 = reduce96s(res0_s);
        let reduced1 = reduce96s(res1_s);
        let reduced2 = reduce96s(res2_s);
        [
            extract::<0>(reduced0),
            extract::<1>(reduced0),
            extract::<2>(reduced0),
            extract::<3>(reduced0),
            extract::<0>(reduced1),
            extract::<1>(reduced1),
            extract::<2>(reduced1),
            extract::<3>(reduced1),
            extract::<0>(reduced2),
            extract::<1>(reduced2),
            extract::<2>(reduced2),
            extract::<3>(reduced2),
        ]
    }
}

#[inline(always)]
unsafe fn reduce96s(x_s: Vecs128) -> __m256i {
    let (hi0, lo0_s) = x_s;
    let lo1 = _mm256_mul_epu32(hi0, _mm256_set1_epi64x(EPSILON as i64));
    add_no_canonicalize_64_64s(lo1, lo0_s)
}

#[inline(always)]
unsafe fn add_no_canonicalize_64_64s(x: __m256i, y_s: __m256i) -> __m256i {
    let res_wrapped_s = _mm256_add_epi64(x, y_s);
    let mask = _mm256_cmpgt_epi64(y_s, res_wrapped_s);
    let res_wrapped = _mm256_xor_si256(res_wrapped_s, _mm256_set1_epi64x(SIGN_BIT as i64));
    let wrapback_amt = _mm256_and_si256(mask, _mm256_set1_epi64x(EPSILON as i64));
    let res = _mm256_add_epi64(res_wrapped, wrapback_amt);
    res
}

/// Poseidon constant layer for Crandall. Assumes that every element in round_constants is in
/// 0..CrandallField::ORDER; when this is not true it may return garbage. It's marked unsafe for
/// this reason.
#[inline(always)]
pub unsafe fn crandall_poseidon_const_avx2<const PACKED_WIDTH: usize>(
    state: &mut [CrandallField; 4 * PACKED_WIDTH],
    round_constants: [u64; 4 * PACKED_WIDTH],
) {
    let packed_state = PackedCrandallAVX2::pack_slice_mut(state);
    let packed_round_constants =
        std::slice::from_raw_parts((&round_constants).as_ptr().cast::<__m256i>(), PACKED_WIDTH);
    for i in 0..PACKED_WIDTH {
        packed_state[i] = packed_state[i].add_canonical_u64(packed_round_constants[i]);
    }
}

#[inline(always)]
pub fn crandall_poseidon_sbox_avx2<const PACKED_WIDTH: usize>(
    state: &mut [CrandallField; 4 * PACKED_WIDTH],
) {
    // This function is manually interleaved to maximize instruction-level parallelism.

    let packed_state = PackedCrandallAVX2::pack_slice_mut(state);

    let mut x2 = [PackedCrandallAVX2::zero(); PACKED_WIDTH];
    for i in 0..PACKED_WIDTH {
        x2[i] = packed_state[i].square();
    }

    let mut x3 = [PackedCrandallAVX2::zero(); PACKED_WIDTH];
    let mut x4 = [PackedCrandallAVX2::zero(); PACKED_WIDTH];
    for i in 0..PACKED_WIDTH {
        x3[i] = packed_state[i] * x2[i];
        x4[i] = x2[i].square();
    }

    for i in 0..PACKED_WIDTH {
        packed_state[i] = x3[i] * x4[i];
    }
}

/// 64-bit by 64-bit multiplication with accumulation into a 192-bit integer. The third argument is
/// assumed to be pre-shifted by (1 << 127) + (1 << 63). The result is similarly shifted.
#[inline]
unsafe fn mac64_64_192ss_ss(
    x: __m256i,
    y: __m256i,
    z_ss: (__m256i, __m256i, __m256i),
) -> (__m256i, __m256i, __m256i) {
    let (z_top, z_hi_s, z_lo_s) = z_ss;

    let x_hi = _mm256_srli_epi64(x, 32);
    let y_hi = _mm256_srli_epi64(y, 32);
    let mul_ll = _mm256_mul_epu32(x, y);
    let mul_lh = _mm256_mul_epu32(x, y_hi);
    let mul_hl = _mm256_mul_epu32(x_hi, y);
    let mul_hh = _mm256_mul_epu32(x_hi, y_hi);

    let res_lo0_s = _mm256_add_epi64(z_lo_s, mul_ll);
    let res_lo1_s = _mm256_add_epi64(res_lo0_s, _mm256_slli_epi64(mul_lh, 32));
    let res_lo2_s = _mm256_add_epi64(res_lo1_s, _mm256_slli_epi64(mul_hl, 32));

    // cmpgt returns -1 on true and 0 on false. Hence, the carry values below are set to -1 on
    // overflow and must be subtracted, not added.
    let carry_lo0 = _mm256_cmpgt_epi64(z_lo_s, res_lo0_s);
    let carry_lo1 = _mm256_cmpgt_epi64(res_lo0_s, res_lo1_s);
    let carry_lo2 = _mm256_cmpgt_epi64(res_lo1_s, res_lo2_s);

    let res_hi0_s = _mm256_add_epi64(z_hi_s, mul_hh);
    let res_hi1_s = _mm256_add_epi64(res_hi0_s, _mm256_srli_epi64(mul_lh, 32));
    let res_hi2_s = _mm256_add_epi64(res_hi1_s, _mm256_srli_epi64(mul_hl, 32));
    let res_hi3_s = _mm256_sub_epi64(res_hi2_s, carry_lo0);
    let res_hi4_s = _mm256_sub_epi64(res_hi3_s, carry_lo1);
    let res_hi5_s = _mm256_sub_epi64(res_hi4_s, carry_lo2);

    let carry_hi = _mm256_cmpgt_epi64(z_hi_s, res_hi5_s);

    let res_top = _mm256_sub_epi64(z_top, carry_hi);

    (res_top, res_hi5_s, res_lo2_s)
}

#[inline]
unsafe fn add_with_carry_hi_lo_los_s(
    hi: __m256i,
    lo0: __m256i,
    lo1_s: __m256i,
) -> (__m256i, __m256i) {
    let res_lo_s = _mm256_add_epi64(lo0, lo1_s);
    let carry = _mm256_cmpgt_epi64(lo1_s, res_lo_s);
    let res_hi = _mm256_sub_epi64(hi, carry);
    (res_hi, res_lo_s)
}

#[inline]
unsafe fn fmadd_64_32_64s_s(x: __m256i, y: __m256i, z_s: __m256i) -> (__m256i, __m256i) {
    let x_hi = _mm256_srli_epi64(x, 32);
    let mul_lo = _mm256_mul_epu32(x, y);
    let mul_hi = _mm256_mul_epu32(x_hi, y);
    let (tmp_hi, tmp_lo_s) = add_with_carry_hi_lo_los_s(_mm256_srli_epi64(mul_hi, 32), mul_lo, z_s);
    add_with_carry_hi_lo_los_s(tmp_hi, _mm256_slli_epi64(mul_hi, 32), tmp_lo_s)
}

#[inline]
unsafe fn reduce128s(x_s: (__m256i, __m256i)) -> __m256i {
    let (hi0, lo0_s) = x_s;
    let t_s = fmadd_64_32_64s_s(hi0, _mm256_set1_epi64x(EPSILON as i64), lo0_s);
    reduce96s(t_s)
}

#[inline(always)]
unsafe fn reduce160ss(x_ss: (__m256i, __m256i, __m256i)) -> __m256i {
    let (top0, hi0_s, lo0_s) = x_ss;
    let t_s = reduce96s((top0, hi0_s));
    reduce128s((t_s, lo0_s))
}

#[inline(always)]
#[unroll_for_loops]
pub fn crandall_mds_partial_layer_init<const PACKED_WIDTH: usize>(
    state: &[CrandallField; 4 * PACKED_WIDTH],
    matrix: &[[u64; 4 * PACKED_WIDTH]; 4 * PACKED_WIDTH - 1],
) -> [CrandallField; 4 * PACKED_WIDTH] {
    let mut cumul = unsafe {
        [(
            _mm256_setzero_si256(),
            _mm256_set1_epi64x(SIGN_BIT as i64),
            _mm256_set1_epi64x(SIGN_BIT as i64),
        ); PACKED_WIDTH]
    };

    assert!(4 * PACKED_WIDTH <= 12);
    for r in 1..12 {
        if r < 4 * PACKED_WIDTH {
            unsafe {
                let state_r = _mm256_set1_epi64x(state[r].0 as i64);
                let matrix_row = matrix[r - 1];
                assert!(PACKED_WIDTH <= 3);
                for c in 0..3 {
                    if c < PACKED_WIDTH {
                        let t = _mm256_loadu_si256(
                            matrix_row[4 * c..4 * c + 4].as_ptr().cast::<__m256i>(),
                        );
                        cumul[c] = mac64_64_192ss_ss(state_r, t, cumul[c]);
                    }
                }
            }
        }
    }

    let mut res = [CrandallField::ZERO; 4 * PACKED_WIDTH];
    {
        let packed_res = PackedCrandallAVX2::pack_slice_mut(&mut res[..]);
        assert!(PACKED_WIDTH <= 3);
        for c in 0..3 {
            if c < PACKED_WIDTH {
                packed_res[c] = PackedCrandallAVX2::new(unsafe { reduce160ss(cumul[c]) });
            }
        }
    }
    res[0] = state[0];
    res
}

#[inline(always)]
#[unroll_for_loops]
pub fn crandall_partial_first_constant_layer<const PACKED_WIDTH: usize>(
    state: &mut [CrandallField; 4 * PACKED_WIDTH],
    round_constants: &[u64; 4 * PACKED_WIDTH],
) {
    let packed_state = PackedCrandallAVX2::pack_slice_mut(state);
    assert!(PACKED_WIDTH <= 3);
    for i in 0..3 {
        if i < PACKED_WIDTH {
            let s = &mut packed_state[i];
            unsafe {
                let c = _mm256_loadu_si256(
                    round_constants[4 * i..4 * i + 4].as_ptr().cast::<__m256i>(),
                );
                *s = s.add_canonical_u64(c);
            }
        }
    }
}
