use core::arch::aarch64::*;

use crate::field::crandall_field::CrandallField;
use crate::field::field_types::PrimeField;

const EPSILON: u64 = 0u64.wrapping_sub(CrandallField::ORDER);

const MDS_MATRIX_EXPS8: [i32; 8] = [2, 0, 1, 8, 4, 3, 0, 0];
const MDS_MATRIX_EXPS12: [i32; 12] = [10, 13, 2, 0, 4, 1, 8, 7, 15, 5, 0, 0];

/// Pair of vectors (hi, lo) representing a u128.
type Vecs128 = (uint64x2_t, uint64x2_t);

/// Takes cumul (u128) and x (u64). Returns cumul + (x << SHIFT) as u128.
#[inline(always)]
unsafe fn shift_and_accumulate<const SHIFT: i32>(
    x: uint64x2_t,
    (hi_cumul, lo_cumul): Vecs128,
) -> Vecs128
where
    [(); (63 - SHIFT) as usize]: ,
{
    let x_shifted_lo = vshlq_n_u64::<SHIFT>(x);
    let res_lo = vaddq_u64(lo_cumul, x_shifted_lo);
    let carry = vcgtq_u64(lo_cumul, res_lo);
    // This works around a bug in Rust's NEON intrisics. A shift by 64, even though well-defined
    // in ARM's docs, is considered undefined behavior by LLVM. Instead of shifting by 64 - x, we
    // shift by 1 and then by 63 - x to avoid this UB.
    let tmp_hi = vsraq_n_u64::<{ 63 - SHIFT }>(hi_cumul, vshrq_n_u64::<1>(x));
    let res_hi = vsubq_u64(tmp_hi, carry);
    (res_hi, res_lo)
}

/// Extract state[OFFSET..OFFSET + 2] as a vector. Wraps around the boundary.
#[inline(always)]
unsafe fn get_vector_with_offset<const WIDTH: usize, const OFFSET: usize>(
    state: [CrandallField; WIDTH],
) -> uint64x2_t {
    let lo = vmov_n_u64(state[OFFSET % WIDTH].0);
    let hi = vmov_n_u64(state[(OFFSET + 1) % WIDTH].0);
    vcombine_u64(lo, hi)
}

/// Extract CrandallField element from vector.
#[inline(always)]
unsafe fn extract<const INDEX: i32>(v: uint64x2_t) -> CrandallField {
    CrandallField(vgetq_lane_u64::<INDEX>(v))
}

type StateVecs8 = (Vecs128, Vecs128, Vecs128, Vecs128);

#[inline(always)]
unsafe fn iteration8<const INDEX: usize, const SHIFT: i32>(
    (cumul0, cumul1, cumul2, cumul3): StateVecs8,
    state: [CrandallField; 8],
) -> StateVecs8
// 4 vectors of 2 needed to represent entire state.
where
    [(); INDEX + 2]: ,
    [(); INDEX + 4]: ,
    [(); INDEX + 6]: ,
    [(); (63 - SHIFT) as usize]: ,
{
    // Entire state, rotated by INDEX.
    let state0 = get_vector_with_offset::<8, INDEX>(state);
    let state1 = get_vector_with_offset::<8, { INDEX + 2 }>(state);
    let state2 = get_vector_with_offset::<8, { INDEX + 4 }>(state);
    let state3 = get_vector_with_offset::<8, { INDEX + 6 }>(state);
    (
        shift_and_accumulate::<SHIFT>(state0, cumul0),
        shift_and_accumulate::<SHIFT>(state1, cumul1),
        shift_and_accumulate::<SHIFT>(state2, cumul2),
        shift_and_accumulate::<SHIFT>(state3, cumul3),
    )
}

#[inline(always)]
pub fn crandall_poseidon8_mds_neon(state: [CrandallField; 8]) -> [CrandallField; 8] {
    unsafe {
        let mut res = (
            (vmovq_n_u64(0), vmovq_n_u64(0)),
            (vmovq_n_u64(0), vmovq_n_u64(0)),
            (vmovq_n_u64(0), vmovq_n_u64(0)),
            (vmovq_n_u64(0), vmovq_n_u64(0)),
        );

        res = iteration8::<0, { MDS_MATRIX_EXPS8[0] }>(res, state);
        res = iteration8::<1, { MDS_MATRIX_EXPS8[1] }>(res, state);
        res = iteration8::<2, { MDS_MATRIX_EXPS8[2] }>(res, state);
        res = iteration8::<3, { MDS_MATRIX_EXPS8[3] }>(res, state);
        res = iteration8::<4, { MDS_MATRIX_EXPS8[4] }>(res, state);
        res = iteration8::<5, { MDS_MATRIX_EXPS8[5] }>(res, state);
        res = iteration8::<6, { MDS_MATRIX_EXPS8[6] }>(res, state);
        res = iteration8::<7, { MDS_MATRIX_EXPS8[7] }>(res, state);

        let (res0, res1, res2, res3) = res;
        let reduced0 = reduce96(res0);
        let reduced1 = reduce96(res1);
        let reduced2 = reduce96(res2);
        let reduced3 = reduce96(res3);
        [
            extract::<0>(reduced0),
            extract::<1>(reduced0),
            extract::<0>(reduced1),
            extract::<1>(reduced1),
            extract::<0>(reduced2),
            extract::<1>(reduced2),
            extract::<0>(reduced3),
            extract::<1>(reduced3),
        ]
    }
}

type StateVecs12 = (Vecs128, Vecs128, Vecs128, Vecs128, Vecs128, Vecs128);

#[inline(always)]
unsafe fn iteration12<const INDEX: usize, const SHIFT: i32>(
    (cumul0, cumul1, cumul2, cumul3, cumul4, cumul5): StateVecs12,
    state: [CrandallField; 12],
) -> StateVecs12
// 6 vectors of 2 needed to represent entire state.
where
    [(); INDEX + 2]: ,
    [(); INDEX + 4]: ,
    [(); INDEX + 6]: ,
    [(); INDEX + 8]: ,
    [(); INDEX + 10]: ,
    [(); (63 - SHIFT) as usize]: ,
{
    // Entire state, rotated by INDEX.
    let state0 = get_vector_with_offset::<12, INDEX>(state);
    let state1 = get_vector_with_offset::<12, { INDEX + 2 }>(state);
    let state2 = get_vector_with_offset::<12, { INDEX + 4 }>(state);
    let state3 = get_vector_with_offset::<12, { INDEX + 6 }>(state);
    let state4 = get_vector_with_offset::<12, { INDEX + 8 }>(state);
    let state5 = get_vector_with_offset::<12, { INDEX + 10 }>(state);
    (
        shift_and_accumulate::<SHIFT>(state0, cumul0),
        shift_and_accumulate::<SHIFT>(state1, cumul1),
        shift_and_accumulate::<SHIFT>(state2, cumul2),
        shift_and_accumulate::<SHIFT>(state3, cumul3),
        shift_and_accumulate::<SHIFT>(state4, cumul4),
        shift_and_accumulate::<SHIFT>(state5, cumul5),
    )
}

#[inline(always)]
pub fn crandall_poseidon12_mds_neon(state: [CrandallField; 12]) -> [CrandallField; 12] {
    unsafe {
        let mut res = (
            (vmovq_n_u64(0), vmovq_n_u64(0)),
            (vmovq_n_u64(0), vmovq_n_u64(0)),
            (vmovq_n_u64(0), vmovq_n_u64(0)),
            (vmovq_n_u64(0), vmovq_n_u64(0)),
            (vmovq_n_u64(0), vmovq_n_u64(0)),
            (vmovq_n_u64(0), vmovq_n_u64(0)),
        );

        res = iteration12::<0, { MDS_MATRIX_EXPS12[0] }>(res, state);
        res = iteration12::<1, { MDS_MATRIX_EXPS12[1] }>(res, state);
        res = iteration12::<2, { MDS_MATRIX_EXPS12[2] }>(res, state);
        res = iteration12::<3, { MDS_MATRIX_EXPS12[3] }>(res, state);
        res = iteration12::<4, { MDS_MATRIX_EXPS12[4] }>(res, state);
        res = iteration12::<5, { MDS_MATRIX_EXPS12[5] }>(res, state);
        res = iteration12::<6, { MDS_MATRIX_EXPS12[6] }>(res, state);
        res = iteration12::<7, { MDS_MATRIX_EXPS12[7] }>(res, state);
        res = iteration12::<8, { MDS_MATRIX_EXPS12[8] }>(res, state);
        res = iteration12::<9, { MDS_MATRIX_EXPS12[9] }>(res, state);
        res = iteration12::<10, { MDS_MATRIX_EXPS12[10] }>(res, state);
        res = iteration12::<11, { MDS_MATRIX_EXPS12[11] }>(res, state);

        let (res0, res1, res2, res3, res4, res5) = res;
        let reduced0 = reduce96(res0);
        let reduced1 = reduce96(res1);
        let reduced2 = reduce96(res2);
        let reduced3 = reduce96(res3);
        let reduced4 = reduce96(res4);
        let reduced5 = reduce96(res5);
        [
            extract::<0>(reduced0),
            extract::<1>(reduced0),
            extract::<0>(reduced1),
            extract::<1>(reduced1),
            extract::<0>(reduced2),
            extract::<1>(reduced2),
            extract::<0>(reduced3),
            extract::<1>(reduced3),
            extract::<0>(reduced4),
            extract::<1>(reduced4),
            extract::<0>(reduced5),
            extract::<1>(reduced5),
        ]
    }
}

#[inline(always)]
unsafe fn reduce96(x: Vecs128) -> uint64x2_t {
    let (hi, lo) = x;
    let hi_lo = vmovn_u64(hi); // Extract the low 32 bits of each 64-bit element
    mul_add_no_canonicalize_64_64(hi_lo, vmov_n_u32(EPSILON as u32), lo)
}

#[inline(always)]
unsafe fn mul_add_no_canonicalize_64_64(x: uint32x2_t, y: uint32x2_t, z: uint64x2_t) -> uint64x2_t {
    let res_wrapped = vmlal_u32(z, x, y);
    let mask = vcgtq_u64(z, res_wrapped);
    let res_unwrapped = vaddq_u64(res_wrapped, vmovq_n_u64(EPSILON));
    vbslq_u64(mask, res_unwrapped, res_wrapped)
}
