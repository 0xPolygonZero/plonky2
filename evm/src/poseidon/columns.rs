use std::ops::Range;

use plonky2::field::types::Field;
use plonky2::hash::poseidon;

use crate::cross_table_lookup::Column;

pub(crate) const POSEIDON_SPONGE_WIDTH: usize = poseidon::SPONGE_WIDTH;
pub(crate) const POSEIDON_SPONGE_RATE: usize = poseidon::SPONGE_RATE;
pub(crate) const HALF_N_FULL_ROUNDS: usize = poseidon::HALF_N_FULL_ROUNDS;
pub(crate) const N_PARTIAL_ROUNDS: usize = poseidon::N_PARTIAL_ROUNDS;
pub(crate) const POSEIDON_DIGEST: usize = 4;

/// Registers to hold permutation inputs.
pub fn reg_input_limb(i: usize) -> usize {
    debug_assert!(i < POSEIDON_SPONGE_WIDTH);
    i
}

pub(crate) fn reg_input_capacity(i: usize) -> usize {
    debug_assert!(i < POSEIDON_SPONGE_WIDTH - POSEIDON_SPONGE_RATE);
    POSEIDON_SPONGE_RATE + i
}

pub fn col_input_limb<F: Field>(i: usize) -> Column<F> {
    Column::single(reg_input_limb(i))
}

const START_CUBED: usize = POSEIDON_SPONGE_WIDTH;
/// Holds x^3 for all elements in full rounds.
pub fn reg_cubed_full(round: usize, i: usize) -> usize {
    debug_assert!(i < POSEIDON_SPONGE_WIDTH);
    debug_assert!(round < 2 * HALF_N_FULL_ROUNDS);
    START_CUBED + POSEIDON_SPONGE_WIDTH * round + i
}

const START_OUTPUT_LIMBS: usize = START_CUBED + 2 * HALF_N_FULL_ROUNDS * POSEIDON_SPONGE_WIDTH;

// The output digest is written in two limbs so we can compare it to
// the values in `CpuStark`.
pub fn reg_output_limb(i: usize) -> usize {
    debug_assert!(i < POSEIDON_SPONGE_WIDTH);
    if i < POSEIDON_DIGEST {
        START_OUTPUT_LIMBS + 2 * i
    } else {
        START_OUTPUT_LIMBS + POSEIDON_DIGEST + i
    }
}

pub fn reg_output_capacity(i: usize) -> usize {
    START_OUTPUT_LIMBS + POSEIDON_SPONGE_RATE + POSEIDON_DIGEST + i
}

pub fn reg_output_capacity_range() -> Range<usize> {
    START_OUTPUT_LIMBS + POSEIDON_SPONGE_RATE + POSEIDON_DIGEST
        ..START_OUTPUT_LIMBS + POSEIDON_DIGEST + POSEIDON_SPONGE_WIDTH
}

pub fn reg_output_digest_range() -> Range<usize> {
    START_OUTPUT_LIMBS..START_OUTPUT_LIMBS + 2 * POSEIDON_DIGEST
}
pub fn reg_output_non_digest_range() -> Range<usize> {
    START_OUTPUT_LIMBS + 2 * POSEIDON_DIGEST
        ..START_OUTPUT_LIMBS + POSEIDON_SPONGE_WIDTH + POSEIDON_DIGEST
}
pub fn col_output_limb<F: Field>(i: usize) -> Column<F> {
    Column::single(reg_output_limb(i))
}

const START_CUBED_PARTIAL: usize = START_OUTPUT_LIMBS + POSEIDON_SPONGE_WIDTH + POSEIDON_DIGEST;
/// Holds x^3 for one element in partial rounds.
pub fn reg_cubed_partial(round: usize) -> usize {
    debug_assert!(round < N_PARTIAL_ROUNDS);
    START_CUBED_PARTIAL + round
}

const START_FULL_0: usize = START_CUBED_PARTIAL + N_PARTIAL_ROUNDS;

/// A column which stores the input of the `i`-th S-box of the `round`-th round of the first set
/// of full rounds.
pub(crate) fn full_sbox_0(round: usize, i: usize) -> usize {
    debug_assert!(
        round != 0,
        "First round S-box inputs are not stored as wires"
    );
    debug_assert!(round < HALF_N_FULL_ROUNDS);
    debug_assert!(i < POSEIDON_SPONGE_WIDTH);
    START_FULL_0 + POSEIDON_SPONGE_WIDTH * (round - 1) + i
}

const START_PARTIAL: usize = START_FULL_0 + POSEIDON_SPONGE_WIDTH * (HALF_N_FULL_ROUNDS - 1);

/// A column which stores the input of the S-box of the `round`-th round of the partial rounds.
pub(crate) fn partial_sbox(round: usize) -> usize {
    debug_assert!(round < N_PARTIAL_ROUNDS);
    START_PARTIAL + round
}

const START_FULL_1: usize = START_PARTIAL + N_PARTIAL_ROUNDS;

/// A wire which stores the input of the `i`-th S-box of the `round`-th round of the second set
/// of full rounds.
pub(crate) fn full_sbox_1(round: usize, i: usize) -> usize {
    debug_assert!(round < HALF_N_FULL_ROUNDS);
    debug_assert!(i < POSEIDON_SPONGE_WIDTH);
    START_FULL_1 + POSEIDON_SPONGE_WIDTH * round + i
}

pub(crate) const IS_FULL_INPUT_BLOCK: usize =
    START_FULL_1 + POSEIDON_SPONGE_WIDTH * HALF_N_FULL_ROUNDS;

const IS_FINAL_INPUT_LEN: usize = IS_FULL_INPUT_BLOCK + 1;
pub(crate) fn reg_is_final_input_len(i: usize) -> usize {
    debug_assert!(i < POSEIDON_SPONGE_RATE);
    IS_FINAL_INPUT_LEN + i
}

pub(crate) fn is_final_len_range() -> Range<usize> {
    IS_FINAL_INPUT_LEN..IS_FINAL_INPUT_LEN + POSEIDON_SPONGE_RATE
}

pub(crate) const START_AUX_COLS: usize = IS_FINAL_INPUT_LEN + POSEIDON_SPONGE_RATE;
pub(crate) fn reg_address() -> usize {
    START_AUX_COLS
}
pub(crate) fn reg_timestamp() -> usize {
    START_AUX_COLS + 3
}
pub(crate) fn reg_len() -> usize {
    START_AUX_COLS + 4
}
pub(crate) fn reg_already_absorbed_elements() -> usize {
    START_AUX_COLS + 5
}
pub(crate) const NUM_COLUMNS: usize = START_AUX_COLS + 6;
