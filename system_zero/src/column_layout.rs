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
pub(crate) mod permutation {
    use plonky2::hash::hashing::SPONGE_WIDTH;
    use plonky2::hash::poseidon;

    const START_UNIT: usize = super::COL_STACK_PTR + 1;

    const START_FULL_FIRST: usize = START_UNIT + SPONGE_WIDTH;

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
        START_UNIT + i
    }

    pub const fn col_output(i: usize) -> usize {
        debug_assert!(i < SPONGE_WIDTH);
        col_full_second_after_mds(poseidon::HALF_N_FULL_ROUNDS - 1, i)
    }

    pub(super) const END_UNIT: usize = col_output(SPONGE_WIDTH - 1);
}

//// MEMORY UNITS

//// DECOMPOSITION UNITS
pub(crate) mod decomposition {

    const START_UNITS: usize = super::permutation::END_UNIT + 1;

    const NUM_UNITS: usize = 4;
    /// The number of bits associated with a single decomposition unit.
    const UNIT_BITS: usize = 32;
    /// One column for the value being decomposed, plus one column per bit.
    const UNIT_COLS: usize = 1 + UNIT_BITS;

    pub const fn col_input(unit: usize) -> usize {
        debug_assert!(unit < NUM_UNITS);
        START_UNITS + unit * UNIT_COLS
    }

    pub const fn col_bit(unit: usize, bit: usize) -> usize {
        debug_assert!(unit < NUM_UNITS);
        debug_assert!(bit < UNIT_BITS);
        START_UNITS + unit * UNIT_COLS + 1 + bit
    }

    pub(super) const END_UNITS: usize = START_UNITS + UNIT_COLS * NUM_UNITS;
}

pub(crate) const NUM_COLUMNS: usize = decomposition::END_UNITS;
