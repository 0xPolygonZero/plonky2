#![allow(clippy::assertions_on_constants)]

use std::arch::aarch64::*;
use std::arch::asm;

use static_assertions::const_assert;
use unroll::unroll_for_loops;

use crate::field::field_types::PrimeField;
use crate::field::goldilocks_field::GoldilocksField;
use crate::hash::poseidon::{
    Poseidon, ALL_ROUND_CONSTANTS, HALF_N_FULL_ROUNDS, N_PARTIAL_ROUNDS, N_ROUNDS,
};

// ========================================== CONSTANTS ===========================================

const WIDTH: usize = 12;

// The order below is arbitrary. Repeated coefficients have been removed so these constants fit in
// two registers.
// TODO: ensure this is aligned to 16 bytes (for vector loads), ideally on the same cacheline
const MDS_CONSTS: [u32; 8] = [
    0xffffffff,
    1 << 1,
    1 << 3,
    1 << 5,
    1 << 8,
    1 << 10,
    1 << 12,
    1 << 16,
];

// The round constants to be applied by the second set of full rounds. These are just the usual round constants,
// shifted by one round, with zeros shifted in.
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

// ===================================== COMPILE-TIME CHECKS ======================================

/// The MDS matrix multiplication ASM is specific to the MDS matrix below. We want this file to
/// fail to compile if it has been changed.
#[allow(dead_code)]
const fn check_mds_matrix() -> bool {
    // Can't == two arrays in a const_assert! (:
    let mut i = 0;
    let wanted_matrix_exps = [0, 0, 1, 0, 3, 5, 1, 8, 12, 3, 16, 10];
    while i < WIDTH {
        if <GoldilocksField as Poseidon>::MDS_MATRIX_EXPS[i] != wanted_matrix_exps[i] {
            return false;
        }
        i += 1;
    }
    true
}
const_assert!(check_mds_matrix());

/// The maximum amount by which the MDS matrix will multiply the input.
/// i.e. max(MDS(state)) <= mds_matrix_inf_norm() * max(state).
const fn mds_matrix_inf_norm() -> u64 {
    let mut cumul = 0;
    let mut i = 0;
    while i < WIDTH {
        cumul += 1 << <GoldilocksField as Poseidon>::MDS_MATRIX_EXPS[i];
        i += 1;
    }
    cumul
}

/// Ensure that adding round constants to the low result of the MDS multiplication can never
/// overflow.
#[allow(dead_code)]
const fn check_round_const_bounds_mds() -> bool {
    let max_mds_res = mds_matrix_inf_norm() * (u32::MAX as u64);
    let mut i = WIDTH; // First const layer is handled specially.
    while i < WIDTH * N_ROUNDS {
        if ALL_ROUND_CONSTANTS[i].overflowing_add(max_mds_res).1 {
            return false;
        }
        i += 1;
    }
    true
}
const_assert!(check_round_const_bounds_mds());

/// Ensure that the first WIDTH round constants are in canonical* form. This is required because
/// the first constant layer does not handle double overflow.
/// *: round_const == GoldilocksField::ORDER is safe.
#[allow(dead_code)]
const fn check_round_const_bounds_init() -> bool {
    let mut i = 0;
    while i < WIDTH {
        if ALL_ROUND_CONSTANTS[i] > GoldilocksField::ORDER {
            return false;
        }
        i += 1;
    }
    true
}
const_assert!(check_round_const_bounds_init());

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
    res.wrapping_add(adj) // adj is EPSILON if wraparound occured and 0 otherwise
}

/// Addition of a and (b >> 32) modulo ORDER accounting for wraparound.
#[inline(always)]
unsafe fn sub_with_wraparound_lsr32(a: u64, b: u64) -> u64 {
    let res: u64;
    let adj: u64;
    asm!(
        "subs  {res}, {a}, {b}, lsr #32",
        // Set adj to 0xffffffff if subtraction underflowed and 0 otherwise.
        // 'cc' for 'carry clear'.
        // NB: The CF in ARM subtraction is the opposite of x86: CF set == underflow did not occur.
        "csetm {adj:w}, cc",
        a = in(reg) a,
        b = in(reg) b,
        res = lateout(reg) res,
        adj = lateout(reg) adj,
        options(pure, nomem, nostack),
    );
    res.wrapping_sub(adj) // adj is EPSILON if underflow occured and 0 otherwise.
}

/// Multiplication of the low word (i.e., x as u32) by EPSILON.
#[inline(always)]
unsafe fn mul_epsilon(x: u64) -> u64 {
    let res;
    let epsilon: u64 = 0xffffffff;
    asm!(
        // Use UMULL to save one instruction. The compiler emits two: extract the low word and then multiply.
        "umull {res}, {x:w}, {epsilon:w}",
        x = in(reg) x,
        epsilon = in(reg) epsilon,
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

// ==================================== STANDALONE CONST LAYER =====================================

/// Standalone const layer. Run only once, at the start of round 1. Remaining const layers are fused with the preceeding
/// MDS matrix multiplication.
#[inline(always)]
#[unroll_for_loops]
unsafe fn const_layer_full(
    mut state: [u64; WIDTH],
    round_constants: &[u64; WIDTH],
) -> [u64; WIDTH] {
    assert!(WIDTH == 12);
    for i in 0..12 {
        let rc = round_constants[i];
        // add_with_wraparound is safe, because rc is in canonical form.
        state[i] = add_with_wraparound(state[i], rc);
    }
    state
}

// ========================================== FULL ROUNDS ==========================================

/// Full S-box.
#[inline(always)]
#[unroll_for_loops]
unsafe fn sbox_layer_full(state: [u64; WIDTH]) -> [u64; WIDTH] {
    // This is done in scalar. S-boxes in vector are only slightly slower throughput-wise but have an insane latency
    // (~100 cycles) on the M1.

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

// Aliases for readability. E.g. MDS[5] can be found in mdsv5[MDSI5].
const MDSI2: i32 = 1; // MDS[2] == 1
const MDSI4: i32 = 2; // MDS[4] == 3
const MDSI5: i32 = 3; // MDS[5] == 5
const MDSI6: i32 = 1; // MDS[6] == 1
const MDSI7: i32 = 0; // MDS[7] == 8
const MDSI8: i32 = 2; // MDS[8] == 12
const MDSI9: i32 = 2; // MDS[9] == 3
const MDSI10: i32 = 3; // MDS[10] == 16
const MDSI11: i32 = 1; // MDS[11] == 10

#[inline(always)]
unsafe fn mds_reduce(
    [[cumul0_a, cumul0_b], [cumul1_a, cumul1_b]]: [[uint64x2_t; 2]; 2],
) -> uint64x2_t {
    // mds_consts0 == [0xffffffff, 1 << 1, 1 << 3, 1 << 5]
    let mds_consts0: uint32x4_t = vld1q_u32((&MDS_CONSTS[0..4]).as_ptr().cast::<u32>());

    // Merge accumulators
    let cumul0 = vaddq_u64(cumul0_a, cumul0_b);
    let cumul1 = vaddq_u64(cumul1_a, cumul1_b);

    // Swizzle
    let res_lo = vzip1q_u64(cumul0, cumul1);
    let res_hi = vzip2q_u64(cumul0, cumul1);

    // Reduce from u96
    let res_hi = vsraq_n_u64::<32>(res_hi, res_lo);
    let res_lo = vsliq_n_u64::<32>(res_lo, res_hi);

    // Extract high 32-bits.
    let res_hi_hi = vget_low_u32(vuzp2q_u32(
        vreinterpretq_u32_u64(res_hi),
        vreinterpretq_u32_u64(res_hi),
    ));

    // Multiply by EPSILON and accumulate.
    let res_unadj = vmlal_laneq_u32::<0>(res_lo, res_hi_hi, mds_consts0);
    let res_adj = vcgtq_u64(res_lo, res_unadj);
    vsraq_n_u64::<32>(res_unadj, res_adj)
}

#[inline(always)]
unsafe fn mds_const_layers_full(
    state: [u64; WIDTH],
    round_constants: &[u64; WIDTH],
) -> [u64; WIDTH] {
    // mds_consts0 == [0xffffffff, 1 << 1, 1 << 3, 1 << 5]
    // mds_consts1 == [1 << 8, 1 << 10, 1 << 12, 1 << 16]
    let mds_consts0: uint32x4_t = vld1q_u32((&MDS_CONSTS[0..4]).as_ptr().cast::<u32>());
    let mds_consts1: uint32x4_t = vld1q_u32((&MDS_CONSTS[4..8]).as_ptr().cast::<u32>());

    // Aliases for readability. E.g. MDS[5] can be found in mdsv5[mdsi5]. MDS[0], MDS[1], and
    // MDS[3] are 0, so they are not needed.
    let mdsv2 = mds_consts0; // MDS[2] == 1
    let mdsv4 = mds_consts0; // MDS[4] == 3
    let mdsv5 = mds_consts0; // MDS[5] == 5
    let mdsv6 = mds_consts0; // MDS[6] == 1
    let mdsv7 = mds_consts1; // MDS[7] == 8
    let mdsv8 = mds_consts1; // MDS[8] == 12
    let mdsv9 = mds_consts0; // MDS[9] == 3
    let mdsv10 = mds_consts1; // MDS[10] == 16
    let mdsv11 = mds_consts1; // MDS[11] == 10

    // For i even, we combine state[i] and state[i + 1] into one vector to save on registers.
    // Thus, state1 actually contains state0 and state1 but is only used in the intrinsics that
    // access the high high doubleword.
    let state1: uint32x4_t =
        vreinterpretq_u32_u64(vcombine_u64(vcreate_u64(state[0]), vcreate_u64(state[1])));
    let state3: uint32x4_t =
        vreinterpretq_u32_u64(vcombine_u64(vcreate_u64(state[2]), vcreate_u64(state[3])));
    let state5: uint32x4_t =
        vreinterpretq_u32_u64(vcombine_u64(vcreate_u64(state[4]), vcreate_u64(state[5])));
    let state7: uint32x4_t =
        vreinterpretq_u32_u64(vcombine_u64(vcreate_u64(state[6]), vcreate_u64(state[7])));
    let state9: uint32x4_t =
        vreinterpretq_u32_u64(vcombine_u64(vcreate_u64(state[8]), vcreate_u64(state[9])));
    let state11: uint32x4_t =
        vreinterpretq_u32_u64(vcombine_u64(vcreate_u64(state[10]), vcreate_u64(state[11])));
    // state0 is an alias to the low doubleword of state1. The compiler should use one register for both.
    let state0: uint32x2_t = vget_low_u32(state1);
    let state2: uint32x2_t = vget_low_u32(state3);
    let state4: uint32x2_t = vget_low_u32(state5);
    let state6: uint32x2_t = vget_low_u32(state7);
    let state8: uint32x2_t = vget_low_u32(state9);
    let state10: uint32x2_t = vget_low_u32(state11);

    // Two accumulators per output to hide latency. Each accumulator is a vector of two u64s,
    // containing the result for the low 32 bits and the high 32 bits. Thus, the final result at
    // index i is (cumuli_a[0] + cumuli_b[0]) + (cumuli_a[1] + cumuli_b[1]) * 2**32.

    // Start by loading the round constants.
    let mut cumul0_a = vcombine_u64(vld1_u64(&round_constants[0]), vcreate_u64(0));
    let mut cumul1_a = vcombine_u64(vld1_u64(&round_constants[1]), vcreate_u64(0));
    let mut cumul2_a = vcombine_u64(vld1_u64(&round_constants[2]), vcreate_u64(0));
    let mut cumul3_a = vcombine_u64(vld1_u64(&round_constants[3]), vcreate_u64(0));
    let mut cumul4_a = vcombine_u64(vld1_u64(&round_constants[4]), vcreate_u64(0));
    let mut cumul5_a = vcombine_u64(vld1_u64(&round_constants[5]), vcreate_u64(0));
    let mut cumul6_a = vcombine_u64(vld1_u64(&round_constants[6]), vcreate_u64(0));
    let mut cumul7_a = vcombine_u64(vld1_u64(&round_constants[7]), vcreate_u64(0));
    let mut cumul8_a = vcombine_u64(vld1_u64(&round_constants[8]), vcreate_u64(0));
    let mut cumul9_a = vcombine_u64(vld1_u64(&round_constants[9]), vcreate_u64(0));
    let mut cumul10_a = vcombine_u64(vld1_u64(&round_constants[10]), vcreate_u64(0));
    let mut cumul11_a = vcombine_u64(vld1_u64(&round_constants[11]), vcreate_u64(0));

    // Now the matrix multiplication.
    // MDS exps: [0, 0, 1, 0, 3, 5, 1, 8, 12, 3, 16, 10]
    // out[i] += in[j] << mds[j - i]

    let mut cumul0_b = vshll_n_u32::<0>(state0); // MDS[0]
    let mut cumul1_b = vshll_n_u32::<10>(state0); // MDS[11]
    let mut cumul2_b = vshll_n_u32::<16>(state0); // MDS[10]
    let mut cumul3_b = vshll_n_u32::<3>(state0); // MDS[9]
    let mut cumul4_b = vshll_n_u32::<12>(state0); // MDS[8]
    let mut cumul5_b = vshll_n_u32::<8>(state0); // MDS[7]
    let mut cumul6_b = vshll_n_u32::<1>(state0); // MDS[6]
    let mut cumul7_b = vshll_n_u32::<5>(state0); // MDS[5]
    let mut cumul8_b = vshll_n_u32::<3>(state0); // MDS[4]
    let mut cumul9_b = vshll_n_u32::<0>(state0); // MDS[3]
    let mut cumul10_b = vshll_n_u32::<1>(state0); // MDS[2]
    let mut cumul11_b = vshll_n_u32::<0>(state0); // MDS[1]

    cumul0_a = vaddw_high_u32(cumul0_a, state1); // MDS[1]
    cumul1_a = vaddw_high_u32(cumul1_a, state1); // MDS[0]
    cumul2_a = vmlal_high_laneq_u32::<MDSI11>(cumul2_a, state1, mdsv11); // MDS[11]
    cumul3_a = vmlal_high_laneq_u32::<MDSI10>(cumul3_a, state1, mdsv10); // MDS[10]
    cumul4_a = vmlal_high_laneq_u32::<MDSI9>(cumul4_a, state1, mdsv9); // MDS[9]
    cumul5_a = vmlal_high_laneq_u32::<MDSI8>(cumul5_a, state1, mdsv8); // MDS[8]
    cumul6_a = vmlal_high_laneq_u32::<MDSI7>(cumul6_a, state1, mdsv7); // MDS[7]
    cumul7_a = vmlal_high_laneq_u32::<MDSI6>(cumul7_a, state1, mdsv6); // MDS[6]
    cumul8_a = vmlal_high_laneq_u32::<MDSI5>(cumul8_a, state1, mdsv5); // MDS[5]
    cumul9_a = vmlal_high_laneq_u32::<MDSI4>(cumul9_a, state1, mdsv4); // MDS[4]
    cumul10_a = vaddw_high_u32(cumul10_a, state1); // MDS[3]
    cumul11_a = vmlal_high_laneq_u32::<MDSI2>(cumul11_a, state1, mdsv2); // MDS[2]

    cumul0_b = vmlal_laneq_u32::<MDSI2>(cumul0_b, state2, mdsv2); // MDS[2]
    cumul1_b = vaddw_u32(cumul1_b, state2); // MDS[1]
    cumul2_b = vaddw_u32(cumul2_b, state2); // MDS[0]
    cumul3_b = vmlal_laneq_u32::<MDSI11>(cumul3_b, state2, mdsv11); // MDS[11]
    cumul4_b = vmlal_laneq_u32::<MDSI10>(cumul4_b, state2, mdsv10); // MDS[10]
    cumul5_b = vmlal_laneq_u32::<MDSI9>(cumul5_b, state2, mdsv9); // MDS[9]
    cumul6_b = vmlal_laneq_u32::<MDSI8>(cumul6_b, state2, mdsv8); // MDS[8]
    cumul7_b = vmlal_laneq_u32::<MDSI7>(cumul7_b, state2, mdsv7); // MDS[7]
    cumul8_b = vmlal_laneq_u32::<MDSI6>(cumul8_b, state2, mdsv6); // MDS[6]
    cumul9_b = vmlal_laneq_u32::<MDSI5>(cumul9_b, state2, mdsv5); // MDS[5]
    cumul10_b = vmlal_laneq_u32::<MDSI4>(cumul10_b, state2, mdsv4); // MDS[4]
    cumul11_b = vaddw_u32(cumul11_b, state2); // MDS[3]

    cumul0_a = vaddw_high_u32(cumul0_a, state3); // MDS[3]
    cumul1_a = vmlal_high_laneq_u32::<MDSI2>(cumul1_a, state3, mdsv2); // MDS[2]
    cumul2_a = vaddw_high_u32(cumul2_a, state3); // MDS[1]
    cumul3_a = vaddw_high_u32(cumul3_a, state3); // MDS[0]
    cumul4_a = vmlal_high_laneq_u32::<MDSI11>(cumul4_a, state3, mdsv11); // MDS[11]
    cumul5_a = vmlal_high_laneq_u32::<MDSI10>(cumul5_a, state3, mdsv10); // MDS[10]
    cumul6_a = vmlal_high_laneq_u32::<MDSI9>(cumul6_a, state3, mdsv9); // MDS[9]
    cumul7_a = vmlal_high_laneq_u32::<MDSI8>(cumul7_a, state3, mdsv8); // MDS[8]
    cumul8_a = vmlal_high_laneq_u32::<MDSI7>(cumul8_a, state3, mdsv7); // MDS[7]
    cumul9_a = vmlal_high_laneq_u32::<MDSI6>(cumul9_a, state3, mdsv6); // MDS[6]
    cumul10_a = vmlal_high_laneq_u32::<MDSI5>(cumul10_a, state3, mdsv5); // MDS[5]
    cumul11_a = vmlal_high_laneq_u32::<MDSI4>(cumul11_a, state3, mdsv4); // MDS[4]

    cumul0_b = vmlal_laneq_u32::<MDSI4>(cumul0_b, state4, mdsv4); // MDS[4]
    cumul1_b = vaddw_u32(cumul1_b, state4); // MDS[3]
    cumul2_b = vmlal_laneq_u32::<MDSI2>(cumul2_b, state4, mdsv2); // MDS[2]
    cumul3_b = vaddw_u32(cumul3_b, state4); // MDS[1]
    cumul4_b = vaddw_u32(cumul4_b, state4); // MDS[0]
    cumul5_b = vmlal_laneq_u32::<MDSI11>(cumul5_b, state4, mdsv11); // MDS[11]
    cumul6_b = vmlal_laneq_u32::<MDSI10>(cumul6_b, state4, mdsv10); // MDS[10]
    cumul7_b = vmlal_laneq_u32::<MDSI9>(cumul7_b, state4, mdsv9); // MDS[9]
    cumul8_b = vmlal_laneq_u32::<MDSI8>(cumul8_b, state4, mdsv8); // MDS[8]
    cumul9_b = vmlal_laneq_u32::<MDSI7>(cumul9_b, state4, mdsv7); // MDS[7]
    cumul10_b = vmlal_laneq_u32::<MDSI6>(cumul10_b, state4, mdsv6); // MDS[6]
    cumul11_b = vmlal_laneq_u32::<MDSI5>(cumul11_b, state4, mdsv5); // MDS[5]

    cumul0_a = vmlal_high_laneq_u32::<MDSI5>(cumul0_a, state5, mdsv5); // MDS[5]
    cumul1_a = vmlal_high_laneq_u32::<MDSI4>(cumul1_a, state5, mdsv4); // MDS[4]
    cumul2_a = vaddw_high_u32(cumul2_a, state5); // MDS[3]
    cumul3_a = vmlal_high_laneq_u32::<MDSI2>(cumul3_a, state5, mdsv2); // MDS[2]
    cumul4_a = vaddw_high_u32(cumul4_a, state5); // MDS[1]
    cumul5_a = vaddw_high_u32(cumul5_a, state5); // MDS[0]
    cumul6_a = vmlal_high_laneq_u32::<MDSI11>(cumul6_a, state5, mdsv11); // MDS[11]
    cumul7_a = vmlal_high_laneq_u32::<MDSI10>(cumul7_a, state5, mdsv10); // MDS[10]
    cumul8_a = vmlal_high_laneq_u32::<MDSI9>(cumul8_a, state5, mdsv9); // MDS[9]
    cumul9_a = vmlal_high_laneq_u32::<MDSI8>(cumul9_a, state5, mdsv8); // MDS[8]
    cumul10_a = vmlal_high_laneq_u32::<MDSI7>(cumul10_a, state5, mdsv7); // MDS[7]
    cumul11_a = vmlal_high_laneq_u32::<MDSI6>(cumul11_a, state5, mdsv6); // MDS[6]

    cumul0_b = vmlal_laneq_u32::<MDSI6>(cumul0_b, state6, mdsv6); // MDS[6]
    cumul1_b = vmlal_laneq_u32::<MDSI5>(cumul1_b, state6, mdsv5); // MDS[5]
    cumul2_b = vmlal_laneq_u32::<MDSI4>(cumul2_b, state6, mdsv4); // MDS[4]
    cumul3_b = vaddw_u32(cumul3_b, state6); // MDS[3]
    cumul4_b = vmlal_laneq_u32::<MDSI2>(cumul4_b, state6, mdsv2); // MDS[2]
    cumul5_b = vaddw_u32(cumul5_b, state6); // MDS[1]
    cumul6_b = vaddw_u32(cumul6_b, state6); // MDS[0]
    cumul7_b = vmlal_laneq_u32::<MDSI11>(cumul7_b, state6, mdsv11); // MDS[11]
    cumul8_b = vmlal_laneq_u32::<MDSI10>(cumul8_b, state6, mdsv10); // MDS[10]
    cumul9_b = vmlal_laneq_u32::<MDSI9>(cumul9_b, state6, mdsv9); // MDS[9]
    cumul10_b = vmlal_laneq_u32::<MDSI8>(cumul10_b, state6, mdsv8); // MDS[8]
    cumul11_b = vmlal_laneq_u32::<MDSI7>(cumul11_b, state6, mdsv7); // MDS[7]

    cumul0_a = vmlal_high_laneq_u32::<MDSI7>(cumul0_a, state7, mdsv7); // MDS[7]
    cumul1_a = vmlal_high_laneq_u32::<MDSI6>(cumul1_a, state7, mdsv6); // MDS[6]
    cumul2_a = vmlal_high_laneq_u32::<MDSI5>(cumul2_a, state7, mdsv5); // MDS[5]
    cumul3_a = vmlal_high_laneq_u32::<MDSI4>(cumul3_a, state7, mdsv4); // MDS[4]
    cumul4_a = vaddw_high_u32(cumul4_a, state7); // MDS[3]
    cumul5_a = vmlal_high_laneq_u32::<MDSI2>(cumul5_a, state7, mdsv2); // MDS[2]
    cumul6_a = vaddw_high_u32(cumul6_a, state7); // MDS[1]
    cumul7_a = vaddw_high_u32(cumul7_a, state7); // MDS[0]
    cumul8_a = vmlal_high_laneq_u32::<MDSI11>(cumul8_a, state7, mdsv11); // MDS[11]
    cumul9_a = vmlal_high_laneq_u32::<MDSI10>(cumul9_a, state7, mdsv10); // MDS[10]
    cumul10_a = vmlal_high_laneq_u32::<MDSI9>(cumul10_a, state7, mdsv9); // MDS[9]
    cumul11_a = vmlal_high_laneq_u32::<MDSI8>(cumul11_a, state7, mdsv8); // MDS[8]

    cumul0_b = vmlal_laneq_u32::<MDSI8>(cumul0_b, state8, mdsv8); // MDS[8]
    cumul1_b = vmlal_laneq_u32::<MDSI7>(cumul1_b, state8, mdsv7); // MDS[7]
    cumul2_b = vmlal_laneq_u32::<MDSI6>(cumul2_b, state8, mdsv6); // MDS[6]
    cumul3_b = vmlal_laneq_u32::<MDSI5>(cumul3_b, state8, mdsv5); // MDS[5]
    cumul4_b = vmlal_laneq_u32::<MDSI4>(cumul4_b, state8, mdsv4); // MDS[4]
    cumul5_b = vaddw_u32(cumul5_b, state8); // MDS[3]
    cumul6_b = vmlal_laneq_u32::<MDSI2>(cumul6_b, state8, mdsv2); // MDS[2]
    cumul7_b = vaddw_u32(cumul7_b, state8); // MDS[1]
    cumul8_b = vaddw_u32(cumul8_b, state8); // MDS[0]
    cumul9_b = vmlal_laneq_u32::<MDSI11>(cumul9_b, state8, mdsv11); // MDS[11]
    cumul10_b = vmlal_laneq_u32::<MDSI10>(cumul10_b, state8, mdsv10); // MDS[10]
    cumul11_b = vmlal_laneq_u32::<MDSI9>(cumul11_b, state8, mdsv9); // MDS[9]

    cumul0_a = vmlal_high_laneq_u32::<MDSI9>(cumul0_a, state9, mdsv9); // MDS[9]
    cumul1_a = vmlal_high_laneq_u32::<MDSI8>(cumul1_a, state9, mdsv8); // MDS[8]
    cumul2_a = vmlal_high_laneq_u32::<MDSI7>(cumul2_a, state9, mdsv7); // MDS[7]
    cumul3_a = vmlal_high_laneq_u32::<MDSI6>(cumul3_a, state9, mdsv6); // MDS[6]
    cumul4_a = vmlal_high_laneq_u32::<MDSI5>(cumul4_a, state9, mdsv5); // MDS[5]
    cumul5_a = vmlal_high_laneq_u32::<MDSI4>(cumul5_a, state9, mdsv4); // MDS[4]
    cumul6_a = vaddw_high_u32(cumul6_a, state9); // MDS[3]
    cumul7_a = vmlal_high_laneq_u32::<MDSI2>(cumul7_a, state9, mdsv2); // MDS[2]
    cumul8_a = vaddw_high_u32(cumul8_a, state9); // MDS[1]
    cumul9_a = vaddw_high_u32(cumul9_a, state9); // MDS[0]
    cumul10_a = vmlal_high_laneq_u32::<MDSI11>(cumul10_a, state9, mdsv11); // MDS[11]
    cumul11_a = vmlal_high_laneq_u32::<MDSI10>(cumul11_a, state9, mdsv10); // MDS[10]

    cumul0_b = vmlal_laneq_u32::<MDSI10>(cumul0_b, state10, mdsv10); // MDS[10]
    cumul1_b = vmlal_laneq_u32::<MDSI9>(cumul1_b, state10, mdsv9); // MDS[9]
    cumul2_b = vmlal_laneq_u32::<MDSI8>(cumul2_b, state10, mdsv8); // MDS[8]
    cumul3_b = vmlal_laneq_u32::<MDSI7>(cumul3_b, state10, mdsv7); // MDS[7]
    cumul4_b = vmlal_laneq_u32::<MDSI6>(cumul4_b, state10, mdsv6); // MDS[6]
    cumul5_b = vmlal_laneq_u32::<MDSI5>(cumul5_b, state10, mdsv5); // MDS[5]
    cumul6_b = vmlal_laneq_u32::<MDSI4>(cumul6_b, state10, mdsv4); // MDS[4]
    cumul7_b = vaddw_u32(cumul7_b, state10); // MDS[3]
    cumul8_b = vmlal_laneq_u32::<MDSI2>(cumul8_b, state10, mdsv2); // MDS[2]
    cumul9_b = vaddw_u32(cumul9_b, state10); // MDS[1]
    cumul10_b = vaddw_u32(cumul10_b, state10); // MDS[0]
    cumul11_b = vmlal_laneq_u32::<MDSI11>(cumul11_b, state10, mdsv11); // MDS[11]

    cumul0_a = vmlal_high_laneq_u32::<MDSI11>(cumul0_a, state11, mdsv11); // MDS[11]
    cumul1_a = vmlal_high_laneq_u32::<MDSI10>(cumul1_a, state11, mdsv10); // MDS[10]
    cumul2_a = vmlal_high_laneq_u32::<MDSI9>(cumul2_a, state11, mdsv9); // MDS[9]
    cumul3_a = vmlal_high_laneq_u32::<MDSI8>(cumul3_a, state11, mdsv8); // MDS[8]
    cumul4_a = vmlal_high_laneq_u32::<MDSI7>(cumul4_a, state11, mdsv7); // MDS[7]
    cumul5_a = vmlal_high_laneq_u32::<MDSI6>(cumul5_a, state11, mdsv6); // MDS[6]
    cumul6_a = vmlal_high_laneq_u32::<MDSI5>(cumul6_a, state11, mdsv5); // MDS[5]
    cumul7_a = vmlal_high_laneq_u32::<MDSI4>(cumul7_a, state11, mdsv4); // MDS[4]
    cumul8_a = vaddw_high_u32(cumul8_a, state11); // MDS[3]
    cumul9_a = vmlal_high_laneq_u32::<MDSI2>(cumul9_a, state11, mdsv2); // MDS[2]
    cumul10_a = vaddw_high_u32(cumul10_a, state11); // MDS[1]
    cumul11_a = vaddw_high_u32(cumul11_a, state11); // MDS[0]

    let reduced = [
        mds_reduce([[cumul0_a, cumul0_b], [cumul1_a, cumul1_b]]),
        mds_reduce([[cumul2_a, cumul2_b], [cumul3_a, cumul3_b]]),
        mds_reduce([[cumul4_a, cumul4_b], [cumul5_a, cumul5_b]]),
        mds_reduce([[cumul6_a, cumul6_b], [cumul7_a, cumul7_b]]),
        mds_reduce([[cumul8_a, cumul8_b], [cumul9_a, cumul9_b]]),
        mds_reduce([[cumul10_a, cumul10_b], [cumul11_a, cumul11_b]]),
    ];
    [
        vgetq_lane_u64::<0>(reduced[0]),
        vgetq_lane_u64::<1>(reduced[0]),
        vgetq_lane_u64::<0>(reduced[1]),
        vgetq_lane_u64::<1>(reduced[1]),
        vgetq_lane_u64::<0>(reduced[2]),
        vgetq_lane_u64::<1>(reduced[2]),
        vgetq_lane_u64::<0>(reduced[3]),
        vgetq_lane_u64::<1>(reduced[3]),
        vgetq_lane_u64::<0>(reduced[4]),
        vgetq_lane_u64::<1>(reduced[4]),
        vgetq_lane_u64::<0>(reduced[5]),
        vgetq_lane_u64::<1>(reduced[5]),
    ]
}

// ======================================== PARTIAL ROUNDS =========================================

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

// ========================================== GLUE CODE ===========================================

#[inline(always)]
unsafe fn full_round(state: [u64; 12], round_constants: &[u64; WIDTH]) -> [u64; 12] {
    let state = sbox_layer_full(state);
    mds_const_layers_full(state, round_constants)
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

#[inline(always)]
fn unwrap_state(state: [GoldilocksField; 12]) -> [u64; 12] {
    [
        state[0].0,
        state[1].0,
        state[2].0,
        state[3].0,
        state[4].0,
        state[5].0,
        state[6].0,
        state[7].0,
        state[8].0,
        state[9].0,
        state[10].0,
        state[11].0,
    ]
}

#[inline(always)]
fn wrap_state(state: [u64; 12]) -> [GoldilocksField; 12] {
    [
        GoldilocksField(state[0]),
        GoldilocksField(state[1]),
        GoldilocksField(state[2]),
        GoldilocksField(state[3]),
        GoldilocksField(state[4]),
        GoldilocksField(state[5]),
        GoldilocksField(state[6]),
        GoldilocksField(state[7]),
        GoldilocksField(state[8]),
        GoldilocksField(state[9]),
        GoldilocksField(state[10]),
        GoldilocksField(state[11]),
    ]
}

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

#[inline(always)]
pub unsafe fn sbox_layer(state: &mut [GoldilocksField; WIDTH]) {
    *state = wrap_state(sbox_layer_full(unwrap_state(*state)));
}

#[inline(always)]
pub unsafe fn mds_layer(state: &[GoldilocksField; WIDTH]) -> [GoldilocksField; WIDTH] {
    let state = unwrap_state(*state);
    // We want to do an MDS layer without the constant layer.
    let round_consts = [0u64; WIDTH];
    let state = mds_const_layers_full(state, &round_consts);
    wrap_state(state)
}
