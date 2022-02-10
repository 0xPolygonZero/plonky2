//! Arithmetic unit.

pub(crate) const IS_ADD: usize = super::START_ARITHMETIC;
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

pub(super) const END: usize = super::START_ARITHMETIC + NUM_SHARED_COLS;
