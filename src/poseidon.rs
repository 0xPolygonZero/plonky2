//! Implementation of the Poseidon hash function, as described in
//! https://eprint.iacr.org/2019/458.pdf

use unroll::unroll_for_loops;
use crate::hash::GMIMC_CONSTANTS; // TEMPORARY until we get specific ARC for Poseidon

use crate::field::field::Field;

const W: usize = 12;

// [1024, 8192, 4, 1, 16, 2, 256, 128, 32728, 32, 1, 1]
const MDS_SHF: [u64; W] = [10, 13, 2, 0, 4, 1, 8, 7, 15, 5, 0, 0];
const MDS_MUL: [u64; W] = [9, 7, 4, 1, 16, 2, 256, 128, 3, 32, 1, 1];

#[inline]
#[unroll_for_loops]
fn constant_layer<F: Field>(state: &mut [F; W]) {
    for i in 0..W {
        // FIXME: Using first row of MDS as ARC for now
        state[i] += F::from_canonical_u64(GMIMC_CONSTANTS[i]);
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
fn mds_layer_orig<F: Field>(state: &[F; W]) -> [F; W] {
    let mut result = [F::ZERO; W];

    for r in 0..W {
        for c in 0..W {
            result[r] += F::from_canonical_u64(MDS_MUL[(c + W - r) % W]) * state[c];
        }
    }
    result
}

#[unroll_for_loops]
pub fn poseidon<F: Field>(input: [F; W]) -> [F; W] {
    // 4+4=8 full rounds, 22 partial rounds => 118 S-boxes

    let mut state = input;

    // Full rounds on the first four iterations.
    for _ in 0..4 {
        constant_layer(&mut state);
        sbox_layer(&mut state);
        state = mds_layer(&state);
    }

    // Partial rounds on the middle 22 iterations
    for _ in 0..22 {
        constant_layer(&mut state);
        // TODO: Does it matter which index we use?
        state[0] = state[0].cube();
        state = mds_layer(&state);
    }

    // Full rounds on the last four iterations.
    for _ in 0..4 {
        constant_layer(&mut state);
        sbox_layer(&mut state);
        state = mds_layer(&state);
    }

    state
}
