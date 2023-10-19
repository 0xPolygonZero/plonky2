use plonky2::field::types::Field;

use crate::cross_table_lookup::Column;

pub(crate) const POSEIDON_SPONGE_WIDTH: usize = 12;
pub(crate) const POSEIDON_SPONGE_RATE: usize = 8;
pub(crate) const HALF_N_FULL_ROUNDS: usize = 4;
pub(crate) const N_PARTIAL_ROUNDS: usize = 22;

/// Registers to hold permutation inputs.
pub fn reg_input_limb(i: usize) -> usize {
    debug_assert!(i < POSEIDON_SPONGE_WIDTH);
    i
}

pub fn col_input_limb<F: Field>(i: usize) -> Column<F> {
    Column::single(reg_input_limb(i))
}

/// Holds x^3 for all elements in full rounds.
pub fn reg_cubed_full(round: usize, i: usize) -> usize {
    debug_assert!(i < POSEIDON_SPONGE_WIDTH);
    debug_assert!(round < 2 * HALF_N_FULL_ROUNDS);
    POSEIDON_SPONGE_WIDTH + POSEIDON_SPONGE_WIDTH * round + i
}

const START_POWER_6: usize = POSEIDON_SPONGE_WIDTH + 2 * HALF_N_FULL_ROUNDS * POSEIDON_SPONGE_WIDTH;

/// Holds x^6 for all elements in full rounds.
pub fn reg_power_6_full(round: usize, i: usize) -> usize {
    debug_assert!(i < POSEIDON_SPONGE_WIDTH);
    debug_assert!(round < 2 * HALF_N_FULL_ROUNDS);
    START_POWER_6 + POSEIDON_SPONGE_WIDTH * round + i
}

const START_OUTPUT_LIMBS: usize = START_POWER_6 + 2 * HALF_N_FULL_ROUNDS * POSEIDON_SPONGE_WIDTH;
pub fn reg_output_limb(i: usize) -> usize {
    debug_assert!(i < POSEIDON_SPONGE_WIDTH);

    START_OUTPUT_LIMBS + i
}

pub fn col_output_limb<F: Field>(i: usize) -> Column<F> {
    Column::single(reg_output_limb(i))
}

const START_CUBED_PARTIAL: usize = START_OUTPUT_LIMBS + POSEIDON_SPONGE_WIDTH;
/// Holds x^3 for one element in partial rounds.
pub fn reg_cubed_partial(round: usize) -> usize {
    debug_assert!(round < N_PARTIAL_ROUNDS);
    START_CUBED_PARTIAL + round
}

const START_POWER_6_PARTIAL: usize = START_CUBED_PARTIAL + N_PARTIAL_ROUNDS;
/// Holds x^6 for one element in partial rounds.
pub fn reg_power_6_partial(round: usize) -> usize {
    debug_assert!(round < N_PARTIAL_ROUNDS);
    START_POWER_6_PARTIAL + round
}

const START_FULL_0: usize = START_POWER_6_PARTIAL + N_PARTIAL_ROUNDS;

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

pub(crate) const FILTER: usize = START_FULL_1 + POSEIDON_SPONGE_WIDTH * HALF_N_FULL_ROUNDS;
pub(crate) const NUM_COLUMNS: usize = FILTER + 1;
