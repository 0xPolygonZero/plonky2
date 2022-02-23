//! Permutation unit.

use plonky2::hash::hashing::SPONGE_WIDTH;
use plonky2::hash::poseidon;

const START_FULL_FIRST: usize = super::START_PERMUTATION + SPONGE_WIDTH;

pub const fn col_full_first_mid_sbox(round: usize, i: usize) -> usize {
    debug_assert!(round < poseidon::HALF_N_FULL_ROUNDS);
    debug_assert!(i < SPONGE_WIDTH);
    START_FULL_FIRST + 2 * round * SPONGE_WIDTH + i
}

pub const fn col_full_first_after_mds(round: usize, i: usize) -> usize {
    debug_assert!(round < poseidon::HALF_N_FULL_ROUNDS);
    debug_assert!(i < SPONGE_WIDTH);
    START_FULL_FIRST + (2 * round + 1) * SPONGE_WIDTH + i
}

const START_PARTIAL: usize =
    col_full_first_after_mds(poseidon::HALF_N_FULL_ROUNDS - 1, SPONGE_WIDTH - 1) + 1;

pub const fn col_partial_mid_sbox(round: usize) -> usize {
    debug_assert!(round < poseidon::N_PARTIAL_ROUNDS);
    START_PARTIAL + 2 * round
}

pub const fn col_partial_after_sbox(round: usize) -> usize {
    debug_assert!(round < poseidon::N_PARTIAL_ROUNDS);
    START_PARTIAL + 2 * round + 1
}

const START_FULL_SECOND: usize = col_partial_after_sbox(poseidon::N_PARTIAL_ROUNDS - 1) + 1;

pub const fn col_full_second_mid_sbox(round: usize, i: usize) -> usize {
    debug_assert!(round <= poseidon::HALF_N_FULL_ROUNDS);
    debug_assert!(i < SPONGE_WIDTH);
    START_FULL_SECOND + 2 * round * SPONGE_WIDTH + i
}

pub const fn col_full_second_after_mds(round: usize, i: usize) -> usize {
    debug_assert!(round <= poseidon::HALF_N_FULL_ROUNDS);
    debug_assert!(i < SPONGE_WIDTH);
    START_FULL_SECOND + (2 * round + 1) * SPONGE_WIDTH + i
}

pub const fn col_input(i: usize) -> usize {
    debug_assert!(i < SPONGE_WIDTH);
    super::START_PERMUTATION + i
}

pub const fn col_output(i: usize) -> usize {
    debug_assert!(i < SPONGE_WIDTH);
    col_full_second_after_mds(poseidon::HALF_N_FULL_ROUNDS - 1, i)
}

pub(super) const END: usize = col_output(SPONGE_WIDTH - 1) + 1;
