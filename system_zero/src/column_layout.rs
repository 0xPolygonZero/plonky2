//// CORE REGISTERS

use plonky2::hash::hashing::SPONGE_WIDTH;
use plonky2::hash::poseidon;

//// CORE REGISTERS

/// A cycle counter. Starts at 0; increments by 1.
pub(crate) const COL_CLOCK: usize = 0;

/// A column which contains the values `[0, ... 2^16 - 1]`, potentially with duplicates. Used for
/// 16-bit range checks.
///
/// For ease of verification, we enforce that it must begin with 0 and end with `2^16 - 1`, and each
/// delta must be either 0 or 1.
pub(crate) const COL_RANGE_16: usize = COL_CLOCK + 1;

/// Pointer to the current instruction.
pub(crate) const COL_INSTRUCTION_PTR: usize = COL_RANGE_16 + 1;
/// Pointer to the base of the current call's stack frame.
pub(crate) const COL_FRAME_PTR: usize = COL_INSTRUCTION_PTR + 1;
/// Pointer to the tip of the current call's stack frame.
pub(crate) const COL_STACK_PTR: usize = COL_FRAME_PTR + 1;

//// PERMUTATION UNIT

const START_PERMUTATION_UNIT: usize = COL_STACK_PTR + 1;

pub(crate) const fn col_permutation_full_first(round: usize, i: usize) -> usize {
    debug_assert!(round < poseidon::HALF_N_FULL_ROUNDS);
    debug_assert!(i < SPONGE_WIDTH);
    START_PERMUTATION_UNIT + round * SPONGE_WIDTH + i
}

const START_PERMUTATION_PARTIAL: usize =
    col_permutation_full_first(poseidon::HALF_N_FULL_ROUNDS - 1, SPONGE_WIDTH - 1) + 1;

pub(crate) const fn col_permutation_partial(round: usize) -> usize {
    debug_assert!(round < poseidon::N_PARTIAL_ROUNDS);
    START_PERMUTATION_PARTIAL + round
}

const START_PERMUTATION_FULL_SECOND: usize = COL_STACK_PTR + 1;

pub(crate) const fn col_permutation_full_second(round: usize, i: usize) -> usize {
    debug_assert!(round <= poseidon::HALF_N_FULL_ROUNDS);
    debug_assert!(i < SPONGE_WIDTH);
    START_PERMUTATION_FULL_SECOND + round * SPONGE_WIDTH + i
}

pub(crate) const fn col_permutation_input(i: usize) -> usize {
    col_permutation_full_first(0, i)
}

pub(crate) const fn col_permutation_output(i: usize) -> usize {
    debug_assert!(i < SPONGE_WIDTH);
    col_permutation_full_second(poseidon::HALF_N_FULL_ROUNDS, i)
}

const END_PERMUTATION_UNIT: usize = col_permutation_output(SPONGE_WIDTH - 1) + 1;

//// MEMORY UNITS

//// DECOMPOSITION UNITS

const COL_START_DECOMPOSITION: usize = END_PERMUTATION_UNIT;

const NUM_DECOMPOSITION_UNITS: usize = 4;
/// The number of bits associated with a single decomposition unit.
const DECOMPOSITION_UNIT_BITS: usize = 32;
/// One column for the value being decomposed, plus one column per bit.
const DECOMPOSITION_UNIT_COLS: usize = 1 + DECOMPOSITION_UNIT_BITS;

pub(crate) const fn col_decomposition_input(unit: usize) -> usize {
    debug_assert!(unit < NUM_DECOMPOSITION_UNITS);
    COL_START_DECOMPOSITION + unit * DECOMPOSITION_UNIT_COLS
}

pub(crate) const fn col_decomposition_bit(unit: usize, bit: usize) -> usize {
    debug_assert!(unit < NUM_DECOMPOSITION_UNITS);
    debug_assert!(bit < DECOMPOSITION_UNIT_BITS);
    COL_START_DECOMPOSITION + unit * DECOMPOSITION_UNIT_COLS + 1 + bit
}

const COL_END_DECOMPOSITION: usize =
    COL_START_DECOMPOSITION + DECOMPOSITION_UNIT_COLS * NUM_DECOMPOSITION_UNITS;

pub(crate) const NUM_COLUMNS: usize = COL_END_DECOMPOSITION;
