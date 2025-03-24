#![allow(clippy::assertions_on_constants)]

use core::arch::aarch64::*;
use core::arch::asm;
use core::mem::transmute;

use static_assertions::const_assert;
use unroll::unroll_for_loops;

use crate::field::goldilocks_field::GoldilocksField;
use crate::hash::poseidon::Poseidon;
use crate::util::branch_hint;

// ========================================== CONSTANTS ===========================================

const WIDTH: usize = 12;

const EPSILON: u64 = 0xffffffff;

// The round constants to be applied by the second set of full rounds. These are just the usual
// round constants, shifted by one round, with zeros shifted in.
/*
const fn make_final_round_constants() -> [u64; WIDTH * HALF_N_FULL_ROUNDS] {
    let mut res = [0; WIDTH * HALF_N_FULL_ROUNDS];
    let mut i: usize = 0;
    while i < WIDTH * (HALF_N_FULL_ROUNDS - 1) {
        res[i] = ALL_ROUND_CONSTANTS[i + WIDTH * (HALF_N_FULL_ROUNDS + N_PARTIAL_ROUNDS + 1)];
        i += 1;
    }
    res
}
const FINAL_ROUND_CONSTANTS: [u64; WIDTH * HALF_N_FULL_ROUNDS] = make_final_round_constants();
*/

// ===================================== COMPILE-TIME CHECKS ======================================

/// The MDS matrix multiplication ASM is specific to the MDS matrix below. We want this file to
/// fail to compile if it has been changed.
#[allow(dead_code)]
const fn check_mds_matrix() -> bool {
    // Can't == two arrays in a const_assert! (:
    let mut i = 0;
    let wanted_matrix_circ = [17, 15, 41, 16, 2, 28, 13, 13, 39, 18, 34, 20];
    let wanted_matrix_diag = [8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    while i < WIDTH {
        if <GoldilocksField as Poseidon>::MDS_MATRIX_CIRC[i] != wanted_matrix_circ[i]
            || <GoldilocksField as Poseidon>::MDS_MATRIX_DIAG[i] != wanted_matrix_diag[i]
        {
            return false;
        }
        i += 1;
    }
    true
}
const_assert!(check_mds_matrix());

// ====================================== SCALAR ARITHMETIC =======================================

/// Addition modulo ORDER accounting for wraparound. Correct only when a + b < 2**64 + ORDER.
#[inline(always)]
unsafe fn add_with_wraparound(a: u64, b: u64) -> u64 {
    let res: u64;
    let adj: u64;
    asm!(
        "adds  {res}, {a}, {b}",
        // Set adj to 0xffffffff if addition overflowed and 0 otherwise.
        // 'cs' for 'carry set'.
        "csetm {adj:w}, cs",
        a = in(reg) a,
        b = in(reg) b,
        res = lateout(reg) res,
        adj = lateout(reg) adj,
        options(pure, nomem, nostack),
    );
    res + adj // adj is EPSILON if wraparound occurred and 0 otherwise
}

/// Subtraction of a and (b >> 32) modulo ORDER accounting for wraparound.
#[inline(always)]
unsafe fn sub_with_wraparound_lsr32(a: u64, b: u64) -> u64 {
    let mut b_hi = b >> 32;
    // Make sure that LLVM emits two separate instructions for the shift and the subtraction. This
    // reduces pressure on the execution units with access to the flags, as they are no longer
    // responsible for the shift. The hack is to insert a fake computation between the two
    // instructions with an `asm` block to make LLVM think that they can't be merged.
    asm!(
        "/* {0} */", // Make Rust think we're using the register.
        inlateout(reg) b_hi,
        options(nomem, nostack, preserves_flags, pure),
    );
    // This could be done with a.overflowing_add(b_hi), but `checked_sub` signals to the compiler
    // that overflow is unlikely (note: this is a standard library implementation detail, not part
    // of the spec).
    match a.checked_sub(b_hi) {
        Some(res) => res,
        None => {
            // Super rare. Better off branching.
            branch_hint();
            let res_wrapped = a.wrapping_sub(b_hi);
            res_wrapped - EPSILON
        }
    }
}

/// Multiplication of the low word (i.e., x as u32) by EPSILON.
#[inline(always)]
unsafe fn mul_epsilon(x: u64) -> u64 {
    let res;
    asm!(
        // Use UMULL to save one instruction. The compiler emits two: extract the low word and then
        // multiply.
        "umull {res}, {x:w}, {epsilon:w}",
        x = in(reg) x,
        epsilon = in(reg) EPSILON,
        res = lateout(reg) res,
        options(pure, nomem, nostack, preserves_flags),
    );
    res
}

#[inline(always)]
unsafe fn multiply(x: u64, y: u64) -> u64 {
    let xy = (x as u128) * (y as u128);
    let xy_lo = xy as u64;
    let xy_hi = (xy >> 64) as u64;

    let res0 = sub_with_wraparound_lsr32(xy_lo, xy_hi);

    let xy_hi_lo_mul_epsilon = mul_epsilon(xy_hi);

    // add_with_wraparound is safe, as xy_hi_lo_mul_epsilon <= 0xfffffffe00000001 <= ORDER.
    add_with_wraparound(res0, xy_hi_lo_mul_epsilon)
}

// ========================================== FULL ROUNDS ==========================================

/// Full S-box.
#[inline(always)]
#[unroll_for_loops]
unsafe fn sbox_layer_full(state: [u64; WIDTH]) -> [u64; WIDTH] {
    // This is done in scalar. S-boxes in vector are only slightly slower throughput-wise but have
    // an insane latency (~100 cycles) on the M1.

    let mut state2 = [0u64; WIDTH];
    assert!(WIDTH == 12);
    for i in 0..12 {
        state2[i] = multiply(state[i], state[i]);
    }

    let mut state3 = [0u64; WIDTH];
    let mut state4 = [0u64; WIDTH];
    assert!(WIDTH == 12);
    for i in 0..12 {
        state3[i] = multiply(state[i], state2[i]);
        state4[i] = multiply(state2[i], state2[i]);
    }

    let mut state7 = [0u64; WIDTH];
    assert!(WIDTH == 12);
    for i in 0..12 {
        state7[i] = multiply(state3[i], state4[i]);
    }

    state7
}

#[inline(always)]
unsafe fn mds_reduce(
    // `cumul_a` and `cumul_b` represent two separate field elements. We take advantage of
    // vectorization by reducing them simultaneously.
    [cumul_a, cumul_b]: [uint32x4_t; 2],
) -> uint64x2_t {
    // Form:
    // `lo = [cumul_a[0] + cumul_a[2] * 2**32, cumul_b[0] + cumul_b[2] * 2**32]`
    // `hi = [cumul_a[1] + cumul_a[3] * 2**32, cumul_b[1] + cumul_b[3] * 2**32]`
    // Observe that the result `== lo + hi * 2**16 (mod Goldilocks)`.
    let mut lo = vreinterpretq_u64_u32(vuzp1q_u32(cumul_a, cumul_b));
    let mut hi = vreinterpretq_u64_u32(vuzp2q_u32(cumul_a, cumul_b));
    // Add the high 48 bits of `lo` to `hi`. This cannot overflow.
    hi = vsraq_n_u64::<16>(hi, lo);
    // Now, result `== lo.bits[0..16] + hi * 2**16 (mod Goldilocks)`.
    // Set the high 48 bits of `lo` to the low 48 bits of `hi`.
    lo = vsliq_n_u64::<16>(lo, hi);
    // At this point, result `== lo + hi.bits[48..64] * 2**64 (mod Goldilocks)`.
    // It remains to fold `hi.bits[48..64]` into `lo`.
    let top = {
        // Extract the top 16 bits of `hi` as a `u32`.
        // Interpret `hi` as a vector of bytes, so we can use a table lookup instruction.
        let hi_u8 = vreinterpretq_u8_u64(hi);
        // Indices defining the permutation. `0xff` is out of bounds, producing `0`.
        let top_idx =
            transmute::<[u8; 8], uint8x8_t>([0x06, 0x07, 0xff, 0xff, 0x0e, 0x0f, 0xff, 0xff]);
        let top_u8 = vqtbl1_u8(hi_u8, top_idx);
        vreinterpret_u32_u8(top_u8)
    };
    // result `== lo + top * 2**64 (mod Goldilocks)`.
    let adj_lo = vmlal_n_u32(lo, top, EPSILON as u32);
    let wraparound_mask = vcgtq_u64(lo, adj_lo);
    vsraq_n_u64::<32>(adj_lo, wraparound_mask) // Add epsilon on overflow.
}

#[inline(always)]
unsafe fn mds_layer_full(state: [u64; WIDTH]) -> [u64; WIDTH] {
    // This function performs an MDS multiplication in complex FFT space.
    // However, instead of performing a width-12 FFT, we perform three width-4 FFTs, which is
    // cheaper. The 12x12 matrix-vector multiplication (a convolution) becomes two 3x3 real
    // matrix-vector multiplications and one 3x3 complex matrix-vector multiplication.

    // We split each 64-bit into four chunks of 16 bits. To prevent overflow, each chunk is 32 bits
    // long. Each NEON vector below represents one field element and consists of four 32-bit chunks:
    // `elem == vector[0] + vector[1] * 2**16 + vector[2] * 2**32 + vector[3] * 2**48`.

    // Constants that we multiply by.
    let mut consts: uint32x4_t = transmute::<[u32; 4], _>([2, 4, 8, 16]);

    // Prevent LLVM from turning fused multiply (by power of 2)-add (1 instruction) into shift and
    // add (two instructions). This fake `asm` block means that LLVM no longer knows the contents of
    // `consts`.
    asm!("/* {0:v} */", // Make Rust think the register is being used.
         inout(vreg) consts,
         options(pure, nomem, nostack, preserves_flags),
    );

    // Four length-3 complex FFTs.
    let mut state_fft = [vdupq_n_u32(0); 12];
    for i in 0..3 {
        // Interpret each field element as a 4-vector of `u16`s.
        let x0 = vcreate_u16(state[i]);
        let x1 = vcreate_u16(state[i + 3]);
        let x2 = vcreate_u16(state[i + 6]);
        let x3 = vcreate_u16(state[i + 9]);

        // `vaddl_u16` and `vsubl_u16` yield 4-vectors of `u32`s.
        let y0 = vaddl_u16(x0, x2);
        let y1 = vaddl_u16(x1, x3);
        let y2 = vsubl_u16(x0, x2);
        let y3 = vsubl_u16(x1, x3);

        let z0 = vaddq_u32(y0, y1);
        let z1 = vsubq_u32(y0, y1);
        let z2 = y2;
        let z3 = y3;

        // The FFT is `[z0, z2 + z3 i, z1, z2 - z3 i]`.

        state_fft[i] = z0;
        state_fft[i + 3] = z1;
        state_fft[i + 6] = z2;
        state_fft[i + 9] = z3;
    }

    // 3x3 real matrix-vector mul for component 0 of the FFTs.
    // Multiply the vector `[x0, x1, x2]` by the matrix
    // `[[ 64,  64, 128],`
    // ` [128,  64,  64],`
    // ` [ 64, 128,  64]]`
    // The results are divided by 4 (this ends up cancelling out some later computations).
    {
        let x0 = state_fft[0];
        let x1 = state_fft[1];
        let x2 = state_fft[2];

        let t = vshlq_n_u32::<4>(x0);
        let u = vaddq_u32(x1, x2);

        let y0 = vshlq_n_u32::<4>(u);
        let y1 = vmlaq_laneq_u32::<3>(t, x2, consts);
        let y2 = vmlaq_laneq_u32::<3>(t, x1, consts);

        state_fft[0] = vaddq_u32(y0, y1);
        state_fft[1] = vaddq_u32(y1, y2);
        state_fft[2] = vaddq_u32(y0, y2);
    }

    // 3x3 real matrix-vector mul for component 2 of the FFTs.
    // Multiply the vector `[x0, x1, x2]` by the matrix
    // `[[ -4,  -8,  32],`
    // ` [-32,  -4,  -8],`
    // ` [  8, -32,  -4]]`
    // The results are divided by 4 (this ends up cancelling out some later computations).
    {
        let x0 = state_fft[3];
        let x1 = state_fft[4];
        let x2 = state_fft[5];
        state_fft[3] = vmlsq_laneq_u32::<2>(vmlaq_laneq_u32::<0>(x0, x1, consts), x2, consts);
        state_fft[4] = vmlaq_laneq_u32::<0>(vmlaq_laneq_u32::<2>(x1, x0, consts), x2, consts);
        state_fft[5] = vmlsq_laneq_u32::<0>(x2, vmlsq_laneq_u32::<1>(x0, x1, consts), consts);
    }

    // 3x3 complex matrix-vector mul for components 1 and 3 of the FFTs.
    // Multiply the vector `[x0r + x0i i, x1r + x1i i, x2r + x2i i]` by the matrix
    // `[[ 4 +  2i,  2 + 32i,  2 -  8i],`
    // ` [-8 -  2i,  4 +  2i,  2 + 32i],`
    // ` [32 -  2i, -8 -  2i,  4 +  2i]]`
    // The results are divided by 2 (this ends up cancelling out some later computations).
    {
        let x0r = state_fft[6];
        let x1r = state_fft[7];
        let x2r = state_fft[8];

        let x0i = state_fft[9];
        let x1i = state_fft[10];
        let x2i = state_fft[11];

        // real part of result <- real part of input
        let r0rr = vaddq_u32(vmlaq_laneq_u32::<0>(x1r, x0r, consts), x2r);
        let r1rr = vmlaq_laneq_u32::<0>(x2r, vmlsq_laneq_u32::<0>(x1r, x0r, consts), consts);
        let r2rr = vmlsq_laneq_u32::<0>(x2r, vmlsq_laneq_u32::<1>(x1r, x0r, consts), consts);

        // real part of result <- imaginary part of input
        let r0ri = vmlsq_laneq_u32::<1>(vmlaq_laneq_u32::<3>(x0i, x1i, consts), x2i, consts);
        let r1ri = vmlsq_laneq_u32::<3>(vsubq_u32(x0i, x1i), x2i, consts);
        let r2ri = vsubq_u32(vaddq_u32(x0i, x1i), x2i);

        // real part of result (total)
        let r0r = vsubq_u32(r0rr, r0ri);
        let r1r = vaddq_u32(r1rr, r1ri);
        let r2r = vmlaq_laneq_u32::<0>(r2ri, r2rr, consts);

        // imaginary part of result <- real part of input
        let r0ir = vmlsq_laneq_u32::<1>(vmlaq_laneq_u32::<3>(x0r, x1r, consts), x2r, consts);
        let r1ir = vmlaq_laneq_u32::<3>(vsubq_u32(x1r, x0r), x2r, consts);
        let r2ir = vsubq_u32(x2r, vaddq_u32(x0r, x1r));

        // imaginary part of result <- imaginary part of input
        let r0ii = vaddq_u32(vmlaq_laneq_u32::<0>(x1i, x0i, consts), x2i);
        let r1ii = vmlaq_laneq_u32::<0>(x2i, vmlsq_laneq_u32::<0>(x1i, x0i, consts), consts);
        let r2ii = vmlsq_laneq_u32::<0>(x2i, vmlsq_laneq_u32::<1>(x1i, x0i, consts), consts);

        // imaginary part of result (total)
        let r0i = vaddq_u32(r0ir, r0ii);
        let r1i = vaddq_u32(r1ir, r1ii);
        let r2i = vmlaq_laneq_u32::<0>(r2ir, r2ii, consts);

        state_fft[6] = r0r;
        state_fft[7] = r1r;
        state_fft[8] = r2r;

        state_fft[9] = r0i;
        state_fft[10] = r1i;
        state_fft[11] = r2i;
    }

    // Three length-4 inverse FFTs.
    // Normally, such IFFT would divide by 4, but we've already taken care of that.
    for i in 0..3 {
        let z0 = state_fft[i];
        let z1 = state_fft[i + 3];
        let z2 = state_fft[i + 6];
        let z3 = state_fft[i + 9];

        let y0 = vsubq_u32(z0, z1);
        let y1 = vaddq_u32(z0, z1);
        let y2 = z2;
        let y3 = z3;

        let x0 = vaddq_u32(y0, y2);
        let x1 = vaddq_u32(y1, y3);
        let x2 = vsubq_u32(y0, y2);
        let x3 = vsubq_u32(y1, y3);

        state_fft[i] = x0;
        state_fft[i + 3] = x1;
        state_fft[i + 6] = x2;
        state_fft[i + 9] = x3;
    }

    // Perform `res[0] += state[0] * 8` for the diagonal component of the MDS matrix.
    state_fft[0] = vmlal_laneq_u16::<4>(
        state_fft[0],
        vcreate_u16(state[0]),         // Each 16-bit chunk gets zero-extended.
        vreinterpretq_u16_u32(consts), // Hack: these constants fit in `u16s`, so we can bit-cast.
    );

    let mut res_arr = [0; 12];
    for i in 0..6 {
        let res = mds_reduce([state_fft[2 * i], state_fft[2 * i + 1]]);
        res_arr[2 * i] = vgetq_lane_u64::<0>(res);
        res_arr[2 * i + 1] = vgetq_lane_u64::<1>(res);
    }

    res_arr
}

// ======================================== PARTIAL ROUNDS =========================================

/*
#[rustfmt::skip]
macro_rules! mds_reduce_asm {
    ($c0:literal, $c1:literal, $out:literal, $consts:literal) => {
        concat!(
            // Swizzle
            "zip1.2d ", $out, ",", $c0, ",", $c1, "\n", // lo
            "zip2.2d ", $c0, ",", $c0, ",", $c1, "\n", // hi

            // Reduction from u96
            "usra.2d ", $c0, ",", $out, ", #32\n", "sli.2d ", $out, ",", $c0, ", #32\n",
            // Extract high 32-bits.
            "uzp2.4s ", $c0, ",", $c0, ",", $c0, "\n",
            // Multiply by EPSILON and accumulate.
            "mov.16b ", $c1, ",", $out, "\n",
            "umlal.2d ", $out, ",", $c0, ", ", $consts, "[0]\n",
            "cmhi.2d ", $c1, ",", $c1, ",", $out, "\n",
            "usra.2d ", $out, ",", $c1, ", #32",
        )
    };
}

#[inline(always)]
unsafe fn partial_round(
    (state_scalar, state_vector): ([u64; WIDTH], [uint64x2_t; 5]),
    round_constants: &[u64; WIDTH],
) -> ([u64; WIDTH], [uint64x2_t; 5]) {
    // see readme-asm.md

    // mds_consts0 == [0xffffffff, 1 << 1, 1 << 3, 1 << 5]
    // mds_consts1 == [1 << 8, 1 << 10, 1 << 12, 1 << 16]
    let mds_consts0: uint32x4_t = vld1q_u32((&MDS_CONSTS[0..4]).as_ptr().cast::<u32>());
    let mds_consts1: uint32x4_t = vld1q_u32((&MDS_CONSTS[4..8]).as_ptr().cast::<u32>());

    let res0: u64;
    let res1: u64;
    let res23: uint64x2_t;
    let res45: uint64x2_t;
    let res67: uint64x2_t;
    let res89: uint64x2_t;
    let res1011: uint64x2_t;

    let res2_scalar: u64;
    let res3_scalar: u64;
    let res4_scalar: u64;
    let res5_scalar: u64;
    let res6_scalar: u64;
    let res7_scalar: u64;
    let res8_scalar: u64;
    let res9_scalar: u64;
    let res10_scalar: u64;
    let res11_scalar: u64;

    asm!(
        "ldp d0, d1, [{rc_ptr}, #16]",
        "fmov   d21, {s1}",
        "ldp    {lo0}, {lo1}, [{rc_ptr}]",
        "umulh  {t0}, {s0}, {s0}",
        "mul    {t1}, {s0}, {s0}",
        "subs   {t1}, {t1}, {t0}, lsr #32",
        "csetm  {t2:w}, cc",
        "lsl    {t3}, {t0}, #32",
        "sub    {t1}, {t1}, {t2}",
        "mov    {t0:w}, {t0:w}",
        "sub    {t0}, {t3}, {t0}",
        "adds   {t0}, {t1}, {t0}",
        "csetm  {t1:w}, cs",
        "add    {t0}, {t0}, {t1}",
        "umulh  {t1}, {s0}, {t0}",
        "umulh  {t2}, {t0}, {t0}",
        "mul    {s0}, {s0}, {t0}",
        "mul    {t0}, {t0}, {t0}",
        "subs   {s0}, {s0}, {t1}, lsr #32",
        "csetm  {t3:w}, cc",
        "subs   {t0}, {t0}, {t2}, lsr #32",
        "csetm  {t4:w}, cc",
        "lsl    {t5}, {t1}, #32",
        "lsl    {t6}, {t2}, #32",
        "sub    {s0}, {s0}, {t3}",
        "sub    {t0}, {t0}, {t4}",
        "mov    {t1:w}, {t1:w}",
        "mov    {t2:w}, {t2:w}",
        "sub    {t1}, {t5}, {t1}",
        "ushll.2d   v10, v21, #10",
        "sub    {t2}, {t6}, {t2}",
        "ushll.2d   v11, v21, #16",
        "adds   {t1}, {s0}, {t1}",
        "uaddw.2d   v0, v0, v22",
        "csetm  {s0:w}, cs",
        "umlal.2d   v1, v22, v31[1]",
        "adds   {t2}, {t0}, {t2}",
        "uaddw2.2d  v10, v10, v22",
        "csetm  {t0:w}, cs",
        "uaddw2.2d  v11, v11, v22",
        "add    {t1}, {t1}, {s0}",
        "ldp d2, d3, [{rc_ptr}, #32]",
        "add    {t2}, {t2}, {t0}",
        "ushll.2d   v12, v21, #3",
        "umulh  {s0}, {t1}, {t2}",
        "ushll.2d   v13, v21, #12",
        "mul    {t0}, {t1}, {t2}",
        "umlal.2d   v0, v23, v30[1]",
        "add    {lo1}, {lo1}, {s1:w}, uxtw",
        "uaddw2.2d  v10, v10, v23",
        "add    {lo0}, {lo0}, {s1:w}, uxtw",
        "uaddw.2d   v11, v11, v23",
        "lsr    {hi0}, {s1}, #32",
        "umlal2.2d  v1, v23, v30[1]",
        "lsr    {t3}, {s2}, #32",
        "umlal.2d   v2, v22, v31[3]",
        "lsr    {t4}, {s3}, #32",
        "umlal2.2d  v12, v22, v31[1]",
        "add    {hi1}, {hi0}, {t3}",
        "umlal.2d   v3, v22, v30[2]",
        "add    {hi0}, {hi0}, {t3}, lsl #1",
        "umlal2.2d  v13, v22, v31[3]",
        "add    {lo1}, {lo1}, {s2:w}, uxtw",
        "ldp d4, d5, [{rc_ptr}, #48]",
        "add    {lo0}, {lo0}, {s2:w}, uxtw #1",
        "ushll.2d   v14, v21, #8",
        "lsr    {t3}, {s4}, #32",
        "ushll.2d   v15, v21, #1",
        "lsr    {t5}, {s5}, #32",
        "umlal.2d   v0, v24, v30[2]",
        "subs   {t0}, {t0}, {s0}, lsr #32",
        "umlal2.2d  v10, v24, v30[3]",
        "add    {hi1}, {hi1}, {t4}, lsl #1",
        "umlal2.2d  v11, v24, v30[2]",
        "add    {t6}, {t3}, {t5}, lsl #3",
        "uaddw.2d   v1, v1, v24",
        "add    {t5}, {t3}, {t5}, lsl #2",
        "uaddw.2d   v2, v2, v23",
        "lsr    {t3}, {s6}, #32",
        "umlal.2d   v3, v23, v31[1]",
        "lsr    {s1}, {s7}, #32",
        "uaddw2.2d  v12, v12, v23",
        "mov    {s2:w}, {s4:w}",
        "uaddw2.2d  v13, v13, v23",
        "add    {hi0}, {hi0}, {t4}",
        "umlal.2d   v4, v22, v31[2]",
        "add    {lo1}, {lo1}, {s3:w}, uxtw #1",
        "umlal2.2d  v14, v22, v30[2]",
        "add    {lo0}, {lo0}, {s3:w}, uxtw",
        "umlal.2d   v5, v22, v31[0]",
        "add    {t4}, {s2}, {s5:w}, uxtw #3",
        "umlal2.2d  v15, v22, v31[2]",
        "add    {s2}, {s2}, {s5:w}, uxtw #2",
        "ldp d6, d7, [{rc_ptr}, #64]",
        "add    {s3}, {s1}, {t3}, lsl #4",
        "ushll.2d   v16, v21, #5",
        "csetm  {t1:w}, cc",
        "ushll.2d   v17, v21, #3",
        "add    {hi1}, {hi1}, {t6}",
        "umlal.2d   v0, v25, v30[1]",
        "add    {hi0}, {hi0}, {t5}, lsl #3",
        "umlal2.2d  v10, v25, v31[0]",
        "mov    {t5:w}, {s6:w}",
        "umlal.2d   v1, v25, v30[3]",
        "mov    {t6:w}, {s7:w}",
        "umlal2.2d  v11, v25, v30[1]",
        "add    {s4}, {t6}, {t5}, lsl #4",
        "umlal.2d   v2, v24, v30[1]",
        "add    {t3}, {t3}, {s1}, lsl #7",
        "uaddw2.2d  v12, v12, v24",
        "lsr    {s1}, {s8}, #32",
        "uaddw.2d   v13, v13, v24",
        "lsr    {s5}, {s9}, #32",
        "umlal2.2d  v3, v24, v30[1]",
        "lsl    {t2}, {s0}, #32",
        "umlal.2d   v4, v23, v31[3]",
        "sub    {t0}, {t0}, {t1}",
        "umlal2.2d  v14, v23, v31[1]",
        "add    {lo1}, {lo1}, {t4}",
        "umlal.2d   v5, v23, v30[2]",
        "add    {lo0}, {lo0}, {s2}, lsl #3",
        "umlal2.2d  v15, v23, v31[3]",
        "add    {t4}, {t5}, {t6}, lsl #7",
        "umlal.2d   v6, v22, v30[1]",
        "add    {hi1}, {hi1}, {s3}, lsl #1",
        "umlal2.2d  v16, v22, v31[0]",
        "add    {t5}, {s1}, {s5}, lsl #4",
        "umlal.2d   v7, v22, v30[3]",
        "mov    {s0:w}, {s0:w}",
        "umlal2.2d  v17, v22, v30[1]",
        "sub    {s0}, {t2}, {s0}",
        "ldp d8, d9, [{rc_ptr}, #80]",
        "add    {lo1}, {lo1}, {s4}, lsl #1",
        "ushll.2d   v18, v21, #0",
        "add    {hi0}, {hi0}, {t3}, lsl #1",
        "ushll.2d   v19, v21, #1",
        "mov    {t3:w}, {s9:w}",
        "umlal.2d   v0, v26, v31[2]",
        "mov    {t6:w}, {s8:w}",
        "umlal2.2d  v10, v26, v30[2]",
        "add    {s2}, {t6}, {t3}, lsl #4",
        "umlal.2d   v1, v26, v31[0]",
        "add    {s1}, {s5}, {s1}, lsl #9",
        "umlal2.2d  v11, v26, v31[2]",
        "lsr    {s3}, {s10}, #32",
        "umlal.2d   v2, v25, v30[2]",
        "lsr    {s4}, {s11}, #32",
        "umlal2.2d  v12, v25, v30[3]",
        "adds   {s0}, {t0}, {s0}",
        "umlal2.2d  v13, v25, v30[2]",
        "add    {lo0}, {lo0}, {t4}, lsl #1",
        "uaddw.2d   v3, v3, v25",
        "add    {t3}, {t3}, {t6}, lsl #9",
        "uaddw.2d   v4, v4, v24",
        "add    {hi1}, {hi1}, {t5}, lsl #8",
        "umlal.2d   v5, v24, v31[1]",
        "add    {t4}, {s3}, {s4}, lsl #13",
        "uaddw2.2d  v14, v14, v24",
        "csetm  {t0:w}, cs",
        "uaddw2.2d  v15, v15, v24",
        "add    {lo1}, {lo1}, {s2}, lsl #8",
        "umlal.2d   v6, v23, v31[2]",
        "add    {hi0}, {hi0}, {s1}, lsl #3",
        "umlal2.2d  v16, v23, v30[2]",
        "mov    {t5:w}, {s10:w}",
        "umlal.2d   v7, v23, v31[0]",
        "mov    {t6:w}, {s11:w}",
        "umlal2.2d  v17, v23, v31[2]",
        "add    {s1}, {t5}, {t6}, lsl #13",
        "umlal.2d   v8, v22, v30[2]",
        "add    {s2}, {s4}, {s3}, lsl #6",
        "umlal2.2d  v18, v22, v30[3]",
        "add    {s0}, {s0}, {t0}",
        "uaddw.2d   v9, v9, v22",
        "add    {lo0}, {lo0}, {t3}, lsl #3",
        "umlal2.2d  v19, v22, v30[2]",
        "add    {t3}, {t6}, {t5}, lsl #6",
        "add.2d     v0, v0, v10",
        "add    {hi1}, {hi1}, {t4}, lsl #3",
        "add.2d     v1, v1, v11",
        "fmov   d20, {s0}",
        "umlal.2d   v0, v20, v31[3]",
        "add    {lo1}, {lo1}, {s1}, lsl #3",
        "umlal.2d   v1, v20, v30[2]",
        "add    {hi0}, {hi0}, {s2}, lsl #10",
        "zip1.2d    v22, v0, v1",
        "lsr    {t4}, {s0}, #32",
        "zip2.2d    v0, v0, v1",
        "add    {lo0}, {lo0}, {t3}, lsl #10",
        "usra.2d    v0, v22, #32",
        "add    {hi1}, {hi1}, {t4}, lsl #10",
        "sli.2d     v22, v0, #32",
        "mov    {t3:w}, {s0:w}",
        "uzp2.4s    v0, v0, v0",
        "add    {lo1}, {lo1}, {t3}, lsl #10",
        "mov.16b    v1, v22",
        "add    {hi0}, {hi0}, {t4}",
        "umlal.2d   v22, v0, v30[0]",
        "add    {lo0}, {lo0}, {t3}",
        "cmhi.2d    v1, v1, v22",
        "lsl    {t0}, {hi0}, #32",
        "usra.2d    v22, v1, #32",
        "lsl    {t1}, {hi1}, #32",
        "fmov       {s2}, d22",
        "adds   {lo0}, {lo0}, {t0}",
        "fmov.d     {s3}, v22[1]",
        "csetm  {t0:w}, cs",
        "umlal.2d   v2, v26, v30[1]",
        "adds   {lo1}, {lo1}, {t1}",
        "umlal2.2d  v12, v26, v31[0]",
        "csetm  {t1:w}, cs",
        "umlal.2d   v3, v26, v30[3]",
        "and    {t2}, {hi0}, #0xffffffff00000000",
        "umlal2.2d  v13, v26, v30[1]",
        "and    {t3}, {hi1}, #0xffffffff00000000",
        "umlal.2d   v4, v25, v30[1]",
        "lsr    {hi0}, {hi0}, #32",
        "uaddw2.2d  v14, v14, v25",
        "lsr    {hi1}, {hi1}, #32",
        "uaddw.2d   v15, v15, v25",
        "sub    {hi0}, {t2}, {hi0}",
        "umlal2.2d  v5, v25, v30[1]",
        "sub    {hi1}, {t3}, {hi1}",
        "umlal.2d   v6, v24, v31[3]",
        "add    {lo0}, {lo0}, {t0}",
        "umlal2.2d  v16, v24, v31[1]",
        "add    {lo1}, {lo1}, {t1}",
        "umlal.2d   v7, v24, v30[2]",
        "adds   {lo0}, {lo0}, {hi0}",
        "umlal2.2d  v17, v24, v31[3]",
        "csetm  {t0:w}, cs",
        "umlal.2d   v8, v23, v30[1]",
        "adds   {lo1}, {lo1}, {hi1}",
        "umlal2.2d  v18, v23, v31[0]",
        "csetm  {t1:w}, cs",
        "umlal.2d   v9, v23, v30[3]",
        "add    {s0}, {lo0}, {t0}",
        "umlal2.2d  v19, v23, v30[1]",
        "add    {s1}, {lo1}, {t1}",
        "add.2d     v2, v2, v12",
        "add.2d     v3, v3, v13",
        "umlal.2d   v2, v20, v31[2]",
        "umlal.2d   v3, v20, v31[0]",
        mds_reduce_asm!("v2", "v3", "v23", "v30"),
        "fmov       {s4}, d23",
        "fmov.d     {s5}, v23[1]",
        "umlal.2d   v4, v26, v30[2]",
        "umlal2.2d  v14, v26, v30[3]",
        "umlal2.2d  v15, v26, v30[2]",
        "uaddw.2d   v5, v5, v26",
        "uaddw.2d   v6, v6, v25",
        "uaddw2.2d  v16, v16, v25",
        "uaddw2.2d  v17, v17, v25",
        "umlal.2d   v7, v25, v31[1]",
        "umlal.2d   v8, v24, v31[2]",
        "umlal2.2d  v18, v24, v30[2]",
        "umlal.2d   v9, v24, v31[0]",
        "umlal2.2d  v19, v24, v31[2]",
        "add.2d     v4, v4, v14",
        "add.2d     v5, v5, v15",
        "umlal.2d   v4, v20, v30[1]",
        "umlal.2d   v5, v20, v30[3]",
        mds_reduce_asm!("v4", "v5", "v24", "v30"),
        "fmov       {s6}, d24",
        "fmov.d     {s7}, v24[1]",
        "umlal.2d   v6, v26, v30[1]",
        "uaddw2.2d  v16, v16, v26",
        "umlal2.2d  v17, v26, v30[1]",
        "uaddw.2d   v7, v7, v26",
        "umlal.2d   v8, v25, v31[3]",
        "umlal2.2d  v18, v25, v31[1]",
        "umlal.2d   v9, v25, v30[2]",
        "umlal2.2d  v19, v25, v31[3]",
        "add.2d     v6, v6, v16",
        "add.2d     v7, v7, v17",
        "umlal.2d   v6, v20, v30[2]",
        "uaddw.2d   v7, v7, v20",
        mds_reduce_asm!("v6", "v7", "v25", "v30"),
        "fmov       {s8}, d25",
        "fmov.d     {s9}, v25[1]",
        "uaddw.2d   v8, v8, v26",
        "uaddw2.2d  v18, v18, v26",
        "umlal.2d   v9, v26, v31[1]",
        "uaddw2.2d  v19, v19, v26",
        "add.2d     v8, v8, v18",
        "add.2d     v9, v9, v19",
        "umlal.2d   v8, v20, v30[1]",
        "uaddw.2d   v9, v9, v20",
        mds_reduce_asm!("v8", "v9", "v26", "v30"),
        "fmov       {s10}, d26",
        "fmov.d     {s11}, v26[1]",

        // Scalar inputs/outputs
        // s0 is transformed by the S-box
        s0 = inout(reg) state_scalar[0] => res0,
        // s1-s6 double as scratch in the MDS matrix multiplication
        s1 = inout(reg) state_scalar[1] => res1,
        // s2-s11 are copied from the vector inputs/outputs
        s2 = inout(reg) state_scalar[2] => res2_scalar,
        s3 = inout(reg) state_scalar[3] => res3_scalar,
        s4 = inout(reg) state_scalar[4] => res4_scalar,
        s5 = inout(reg) state_scalar[5] => res5_scalar,
        s6 = inout(reg) state_scalar[6] => res6_scalar,
        s7 = inout(reg) state_scalar[7] => res7_scalar,
        s8 = inout(reg) state_scalar[8] => res8_scalar,
        s9 = inout(reg) state_scalar[9] => res9_scalar,
        s10 = inout(reg) state_scalar[10] => res10_scalar,
        s11 = inout(reg) state_scalar[11] => res11_scalar,

        // Pointer to the round constants
        rc_ptr = in(reg) round_constants.as_ptr(),

        // Scalar MDS multiplication accumulators
        lo1 = out(reg) _,
        hi1 = out(reg) _,
        lo0 = out(reg) _,
        hi0 = out(reg) _,

        // Scalar scratch registers
        // All are used in the scalar S-box
        t0 = out(reg) _,
        t1 = out(reg) _,
        t2 = out(reg) _,
        // t3-t6 are used in the scalar MDS matrix multiplication
        t3 = out(reg) _,
        t4 = out(reg) _,
        t5 = out(reg) _,
        t6 = out(reg) _,

        // Vector MDS multiplication accumulators
        // v{n} and v1{n} are accumulators for res[n + 2] (we need two to mask latency)
        // The low and high 64-bits are accumulators for the low and high results, respectively
        out("v0") _,
        out("v1") _,
        out("v2") _,
        out("v3") _,
        out("v4") _,
        out("v5") _,
        out("v6") _,
        out("v7") _,
        out("v8") _,
        out("v9") _,
        out("v10") _,
        out("v11") _,
        out("v12") _,
        out("v13") _,
        out("v14") _,
        out("v15") _,
        out("v16") _,
        out("v17") _,
        out("v18") _,
        out("v19") _,

        // Inputs into vector MDS matrix multiplication
        // v20 and v21 are sbox(state0) and state1, respectively. They are copied from the scalar
        // registers.
        out("v20") _,
        out("v21") _,
        // v22, ..., v26 hold state[2,3], ..., state[10,11]
        inout("v22") state_vector[0] => res23,
        inout("v23") state_vector[1] => res45,
        inout("v24") state_vector[2] => res67,
        inout("v25") state_vector[3] => res89,
        inout("v26") state_vector[4] => res1011,

        // Useful constants
        in("v30") mds_consts0,
        in("v31") mds_consts1,

        options(nostack, pure, readonly),
    );
    (
        [
            res0,
            res1,
            res2_scalar,
            res3_scalar,
            res4_scalar,
            res5_scalar,
            res6_scalar,
            res7_scalar,
            res8_scalar,
            res9_scalar,
            res10_scalar,
            res11_scalar,
        ],
        [res23, res45, res67, res89, res1011],
    )
}
*/

// ========================================== GLUE CODE ===========================================

/*
#[inline(always)]
unsafe fn full_round(state: [u64; 12], round_constants: &[u64; WIDTH]) -> [u64; 12] {
    let state = sbox_layer_full(state);
    mds_layer_full(state, round_constants)
}

#[inline]
unsafe fn full_rounds(
    mut state: [u64; 12],
    round_constants: &[u64; WIDTH * HALF_N_FULL_ROUNDS],
) -> [u64; 12] {
    for round_constants_chunk in round_constants.chunks_exact(WIDTH) {
        state = full_round(state, round_constants_chunk.try_into().unwrap());
    }
    state
}

#[inline(always)]
unsafe fn partial_rounds(
    state: [u64; 12],
    round_constants: &[u64; WIDTH * N_PARTIAL_ROUNDS],
) -> [u64; 12] {
    let mut state = (
        state,
        [
            vcombine_u64(vcreate_u64(state[2]), vcreate_u64(state[3])),
            vcombine_u64(vcreate_u64(state[4]), vcreate_u64(state[5])),
            vcombine_u64(vcreate_u64(state[6]), vcreate_u64(state[7])),
            vcombine_u64(vcreate_u64(state[8]), vcreate_u64(state[9])),
            vcombine_u64(vcreate_u64(state[10]), vcreate_u64(state[11])),
        ],
    );
    for round_constants_chunk in round_constants.chunks_exact(WIDTH) {
        state = partial_round(state, round_constants_chunk.try_into().unwrap());
    }
    state.0
}
*/

#[inline(always)]
fn unwrap_state(state: [GoldilocksField; 12]) -> [u64; 12] {
    state.map(|s| s.0)
}

#[inline(always)]
fn wrap_state(state: [u64; 12]) -> [GoldilocksField; 12] {
    state.map(GoldilocksField)
}

/*
#[inline(always)]
pub unsafe fn poseidon(state: [GoldilocksField; 12]) -> [GoldilocksField; 12] {
    let state = unwrap_state(state);
    let state = const_layer_full(state, ALL_ROUND_CONSTANTS[0..WIDTH].try_into().unwrap());
    let state = full_rounds(
        state,
        ALL_ROUND_CONSTANTS[WIDTH..WIDTH * (HALF_N_FULL_ROUNDS + 1)]
            .try_into()
            .unwrap(),
    );
    let state = partial_rounds(
        state,
        ALL_ROUND_CONSTANTS
            [WIDTH * (HALF_N_FULL_ROUNDS + 1)..WIDTH * (HALF_N_FULL_ROUNDS + N_PARTIAL_ROUNDS + 1)]
            .try_into()
            .unwrap(),
    );
    let state = full_rounds(state, &FINAL_ROUND_CONSTANTS);
    wrap_state(state)
}
*/

#[inline(always)]
pub unsafe fn sbox_layer(state: &mut [GoldilocksField; WIDTH]) {
    *state = wrap_state(sbox_layer_full(unwrap_state(*state)));
}

#[inline(always)]
pub unsafe fn mds_layer(state: &[GoldilocksField; WIDTH]) -> [GoldilocksField; WIDTH] {
    let state = unwrap_state(*state);
    let state = mds_layer_full(state);
    wrap_state(state)
}
