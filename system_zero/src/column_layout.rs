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

//// ARITHMETIC UNIT
pub(crate) mod arithmetic {
    //! Arithmetic unit.
    const START_UNIT: usize = super::COL_STACK_PTR + 1;

    pub(crate) const IS_ADD: usize = START_UNIT;
    pub(crate) const IS_SUB: usize = IS_ADD + 1;
    pub(crate) const IS_MUL: usize = IS_SUB + 1;
    pub(crate) const IS_DIV: usize = IS_MUL + 1;

    const START_SHARED_COLS: usize = IS_DIV + 1;

    /// Within the arithmetic unit, there are shared columns which can be used by any arithmetic
    /// circuit, depending on which one is active this cycle.
    // Can be increased as needed as other operations are implemented.
    const NUM_SHARED_COLS: usize = 3;

    const fn shared_col(i: usize) -> usize {
        debug_assert!(i < NUM_SHARED_COLS);
        START_SHARED_COLS + i
    }

    /// The first value to be added; treated as an unsigned u32.
    pub(crate) const COL_ADD_INPUT_1: usize = shared_col(0);
    /// The second value to be added; treated as an unsigned u32.
    pub(crate) const COL_ADD_INPUT_2: usize = shared_col(1);
    /// The third value to be added; treated as an unsigned u32.
    pub(crate) const COL_ADD_INPUT_3: usize = shared_col(2);

    // Note: Addition outputs three 16-bit chunks, and since these values need to be range-checked
    // anyway, we might as well use the range check unit's columns as our addition outputs. So the
    // three proceeding columns are basically aliases, not columns owned by the arithmetic unit.
    /// The first 16-bit chunk of the output, based on little-endian ordering.
    pub(crate) const COL_ADD_OUTPUT_1: usize = super::range_check_16::col_rc_16_input(0);
    /// The second 16-bit chunk of the output, based on little-endian ordering.
    pub(crate) const COL_ADD_OUTPUT_2: usize = super::range_check_16::col_rc_16_input(1);
    /// The third 16-bit chunk of the output, based on little-endian ordering.
    pub(crate) const COL_ADD_OUTPUT_3: usize = super::range_check_16::col_rc_16_input(2);

    pub(super) const END_UNIT: usize = START_UNIT + NUM_SHARED_COLS - 1;
}

pub(crate) mod logic {
    //! Logic unit.
    const START_UNIT: usize = super::arithmetic::END_UNIT + 1;
    pub(super) const END_UNIT: usize = START_UNIT;
}

pub(crate) mod boolean {
    //! Boolean unit. Contains columns whose values must be 0 or 1.

    const START_UNIT: usize = super::logic::END_UNIT + 1;

    const NUM_BITS: usize = 128;

    pub const fn col_bit(index: usize) -> usize {
        debug_assert!(index < NUM_BITS);
        START_UNIT + index
    }

    pub(super) const END_UNIT: usize = START_UNIT + NUM_BITS - 1;
}

pub(crate) mod range_check_16 {
    //! Range check unit which checks that values are in `[0, 2^16)`.

    const START_UNIT: usize = super::boolean::END_UNIT + 1;
    pub(super) const NUM_RANGE_CHECKS: usize = 5;

    /// The input of the `i`th range check, i.e. the value being range checked.
    pub(crate) const fn col_rc_16_input(i: usize) -> usize {
        debug_assert!(i < NUM_RANGE_CHECKS);
        START_UNIT + i
    }

    pub(super) const END_UNIT: usize = START_UNIT + NUM_RANGE_CHECKS - 1;
}

pub(crate) mod range_check_degree {
    //! Range check unit which checks that values are in `[0, degree)`.

    const START_UNIT: usize = super::range_check_16::END_UNIT + 1;

    pub(super) const NUM_RANGE_CHECKS: usize = 5;

    /// The input of the `i`th range check, i.e. the value being range checked.
    pub(crate) const fn col_rc_degree_input(i: usize) -> usize {
        debug_assert!(i < NUM_RANGE_CHECKS);
        START_UNIT + i
    }

    pub(super) const END_UNIT: usize = START_UNIT + NUM_RANGE_CHECKS - 1;
}

pub(crate) mod lookup {
    //! Lookup unit.
    //! See https://zcash.github.io/halo2/design/proving-system/lookup.html

    const START_UNIT: usize = super::range_check_degree::END_UNIT + 1;

    const NUM_LOOKUPS: usize =
        super::range_check_16::NUM_RANGE_CHECKS + super::range_check_degree::NUM_RANGE_CHECKS;

    /// This column contains a permutation of the input values.
    const fn col_permuted_input(i: usize) -> usize {
        debug_assert!(i < NUM_LOOKUPS);
        START_UNIT + 2 * i
    }

    /// This column contains a permutation of the table values.
    const fn col_permuted_table(i: usize) -> usize {
        debug_assert!(i < NUM_LOOKUPS);
        START_UNIT + 2 * i + 1
    }

    pub(super) const END_UNIT: usize = START_UNIT + NUM_LOOKUPS - 1;
}

pub(crate) mod permutation {
    //! Permutation unit.

    use plonky2::hash::hashing::SPONGE_WIDTH;
    use plonky2::hash::poseidon;

    const START_UNIT: usize = super::lookup::END_UNIT + 1;

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

pub(crate) mod memory {
    //! Memory unit.
}

pub(crate) const NUM_COLUMNS: usize = permutation::END_UNIT + 1;
