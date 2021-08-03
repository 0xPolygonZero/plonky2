//! Implementation of the Poseidon hash function, as described in
//! https://eprint.iacr.org/2019/458.pdf

use unroll::unroll_for_loops;
use crate::hash::{GMIMC_CONSTANTS, GMIMC_ROUNDS}; // TEMPORARY until we get specific ARC for Poseidon

use crate::field::field::Field;

const W: usize = 12;

// [1024, 8192, 4, 1, 16, 2, 256, 128, 32728, 32, 1, 1]
const MDS_SHF: [u64; W] = [10, 13, 2, 0, 4, 1, 8, 7, 15, 5, 0, 0];
const MDS_MUL: [u64; W] = [9, 7, 4, 1, 16, 2, 256, 128, 3, 32, 1, 1];

const MDS_PARTIAL_ROUND_CONSTANTS: [u64; N_ROUNDS] = [];
const MDS_PARTIAL_ROUND_INITAL_MATRIX: [[u64; W - 1]; W - 1] = [[]];
const MDS_PARTIAL_ROUND_TOP_LEFT: u64 = 0;
const MDS_PARTIAL_ROUND_VS: [[u64; W - 1]; N_ROUNDS] = [[]];
const MDS_PARTIAL_ROUND_W_HATS: [[u64; W - 1]; N_ROUNDS] = [[]];

#[inline]
#[unroll_for_loops]
fn constant_layer<F: Field>(state: &mut [F; W], round_ctr: usize) {
    for i in 0..W {
        // FIXME: Using first row of MDS as ARC for now
        state[i] += F::from_canonical_u64(GMIMC_CONSTANTS[(i + W*round_ctr) % GMIMC_ROUNDS]);
    }
}

#[inline]
#[unroll_for_loops]
fn sbox_layer<F: Field>(state: &mut [F; W]) {
    for i in 0..W {
        state[i] = state[i].cube();
    }
}

#[inline]
#[unroll_for_loops]
fn mds_row_shf(r: usize, v: &[u64; W]) -> u128 {
    debug_assert!(r < W);
    // TODO: Double-check that the calculations associated with the
    // zeros in this const array are not removed by the compiler; they
    // weren't removed when I used MDS[(i + r) % W], but they seem to
    // be when using MDS[i].

    let mut res = 0u128;
    for i in 0..W {
        res += (v[(i + W - r) % W] as u128) << MDS_SHF[i];
    }
    res
}

#[inline]
#[unroll_for_loops]
fn mds_row_mul(r: usize, v: &[u64; W]) -> u128 {
    debug_assert!(r < W);
    let mut res = 0u128;
    for i in 0..W {
        res += (v[(i + W - r) % W] as u128) * MDS_MUL[i] as u128;
    }
    res
}

#[inline]
#[unroll_for_loops]
fn mds_layer<F: Field>(state_: &[F; W]) -> [F; W] {
    let mut result = [F::ZERO; W];

    // FIXME: Need a better way to do this; we only want the raw u64 anyway.
    let mut state = [0u64; W];
    for r in 0..W {
        state[r] = state_[r].to_canonical_u64();
    }

    for r in 0..W {
        result[r] = F::from_canonical_u128(mds_row_shf(r, &state));
    }
    result
}

#[inline]
#[unroll_for_loops]
fn mds_partial_layer<F: Field>(state: &[F; W]) -> [F; W] {
    let mut result = [F::ZERO; W];

    // FIXME: Initial matrix has first row/column = [1, 0, ..., 0];
    // incorporate this into the calculation to avoid mul-by-zeros
    for r in 0..W {
        for c in 0..W {
            let t = F::from_canonical_u64(MDS_PARTIAL_ROUND_INITAL_MATRIX[r][c]);
            result[r] += t * state[c];
        }
    }
    result
}

/// Computes s*A where s is the state row vector and A is the matrix
///
///    [ M_00  | v  ]
///    [ ------+--- ]
///    [ w_hat | Id ]
///
/// M_00 is a scalar, v is 1x(t-1), w_hat is (t-1)x1 and Id is the
/// (t-1)x(t-1) identity matrix.
#[inline]
#[unroll_for_loops]
fn mds_partial_layer_fast<F: Field>(state: &[F; W], r: usize) -> [F; W] {
    // Set d = [M_00 | w^] dot [state]
    let mut d = MDS_PARTIAL_ROUND_TOP_LEFT * state[0];
    for i in 1..W {
        d += MDS_PARTIAL_ROUND_W_HATS[r][i] * state[i]
    }

    // result = [d] concat [state[0] * v + state[shift up by 1]]
    let mut result = [F::ZERO; W];
    result[0] = d;
    for i in 1..W {
        result[i] = state[0] * MDS_PARTIAL_ROUND_VS[r][i - 1] + state[i];
    }
    result
}

#[inline]
#[unroll_for_loops]
fn full_rounds<F: Field>(state: &mut [F; W], round_ctr: &mut usize) {
    for _ in 0..4 {
        constant_layer(state, *round_ctr);
        sbox_layer(state);
        *state = mds_layer(state);
        *round_ctr += 1;
    }
}


#[inline]
#[unroll_for_loops]
fn partial_rounds<F: Field>(state: &mut [F; W], round_ctr: &mut usize) {
    for _ in 0..22 {
        constant_layer(state, *round_ctr);
        state[0] = state[0].cube();
        *state = mds_layer(state);
        *round_ctr += 1;
    }
}

#[inline]
#[unroll_for_loops]
fn partial_rounds_fast<F: Field>(state: &mut [F; W], round_ctr: &mut usize) {
    constant_layer(state, *round_ctr);

    *state = mds_partial_layer(state);
    for i in 0..21 {
        state[0] = state[0].cube();
        *round_ctr += 1;
        state[0] += MDS_PARTIAL_ROUND_CONSTANTS[i]; // round_consts[round_ctr][0];
        *state = mds_partial_layer_fast(state, i);
    }
    state[0] = state[0].cube();
    *state = mds_partial_layer_fast(state, 21);
    *round_ctr += 1;
}

#[unroll_for_loops]
pub fn poseidon_fast<F: Field>(input: [F; W]) -> [F; W] {
    // TODO: Make these constant values into parameters
    // 4+4=8 full rounds, 22 partial rounds => 118 S-boxes

    let mut state = input;
    let mut round_ctr = 0;

    full_rounds(&mut state, &mut round_ctr);
    partial_rounds_fast(&mut state, &mut round_ctr);
    full_rounds(&mut state, &mut round_ctr);

    state
}

#[unroll_for_loops]
pub fn poseidon<F: Field>(input: [F; W]) -> [F; W] {
    let mut state = input;
    let mut round_ctr = 0;

    full_rounds(&mut state, &mut round_ctr);
    partial_rounds(&mut state, &mut round_ctr);
    full_rounds(&mut state, &mut round_ctr);

    state
}
