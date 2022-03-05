//! Arithmetic and logic unit.

pub(crate) const IS_ADD: usize = super::START_ALU;
pub(crate) const IS_SUB: usize = IS_ADD + 1;
pub(crate) const IS_MUL: usize = IS_SUB + 1;
pub(crate) const IS_DIV: usize = IS_MUL + 1;

const START_SHARED_COLS: usize = IS_DIV + 1;

/// Within the ALU, there are shared columns which can be used by any arithmetic/logic
/// circuit, depending on which one is active this cycle.
// Can be increased as needed as other operations are implemented.
const NUM_SHARED_COLS: usize = 4;

const fn shared_col(i: usize) -> usize {
    debug_assert!(i < NUM_SHARED_COLS);
    START_SHARED_COLS + i
}

/// The first value to be added; treated as an unsigned u32.
pub(crate) const COL_ADD_INPUT_0: usize = shared_col(0);
/// The second value to be added; treated as an unsigned u32.
pub(crate) const COL_ADD_INPUT_1: usize = shared_col(1);
/// The third value to be added; treated as an unsigned u32.
pub(crate) const COL_ADD_INPUT_2: usize = shared_col(2);

// Note: Addition outputs three 16-bit chunks, and since these values need to be range-checked
// anyway, we might as well use the range check unit's columns as our addition outputs. So the
// three proceeding columns are basically aliases, not columns owned by the ALU.
/// The first 16-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_ADD_OUTPUT_0: usize = super::range_check_16::col_rc_16_input(0);
/// The second 16-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_ADD_OUTPUT_1: usize = super::range_check_16::col_rc_16_input(1);
/// The third 16-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_ADD_OUTPUT_2: usize = super::range_check_16::col_rc_16_input(2);

/// Inputs for subtraction; the second value is subtracted from the
/// first; inputs treated as an unsigned u32.
pub(crate) const COL_SUB_INPUT_0: usize = shared_col(0);
pub(crate) const COL_SUB_INPUT_1: usize = shared_col(1);

/// The first 16-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_SUB_OUTPUT_0: usize = super::range_check_16::col_rc_16_input(0);
/// The second 16-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_SUB_OUTPUT_1: usize = super::range_check_16::col_rc_16_input(1);
/// The borrow output
pub(crate) const COL_SUB_OUTPUT_BORROW: usize = super::boolean::col_bit(0);

/// The first value to be multiplied; treated as an unsigned u32.
pub(crate) const COL_MUL_ADD_FACTOR_0: usize = shared_col(0);
/// The second value to be multiplied; treated as an unsigned u32.
pub(crate) const COL_MUL_ADD_FACTOR_1: usize = shared_col(1);
/// The value to be added to the product; treated as an unsigned u32.
pub(crate) const COL_MUL_ADD_ADDEND: usize = shared_col(2);

/// The inverse of `u32::MAX - result_hi`, where `output_hi` is the high 32-bits of the result.
/// See https://hackmd.io/NC-yRmmtRQSvToTHb96e8Q#Checking-element-validity
pub(crate) const COL_MUL_ADD_RESULT_CANONICAL_INV: usize = shared_col(3);

/// The first 16-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_MUL_ADD_OUTPUT_0: usize = super::range_check_16::col_rc_16_input(0);
/// The second 16-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_MUL_ADD_OUTPUT_1: usize = super::range_check_16::col_rc_16_input(1);
/// The third 16-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_MUL_ADD_OUTPUT_2: usize = super::range_check_16::col_rc_16_input(2);
/// The fourth 16-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_MUL_ADD_OUTPUT_3: usize = super::range_check_16::col_rc_16_input(3);

/// Dividend for division, as an unsigned u32
pub(crate) const COL_DIV_INPUT_DIVIDEND: usize = shared_col(0);
/// Divisor for division, as an unsigned u32
pub(crate) const COL_DIV_INPUT_DIVISOR: usize = shared_col(1);
/// Inverse of the divisor in the prime field (or an arbitrary value if one does not exist).
pub(crate) const COL_DIV_DIVISOR_INV: usize = shared_col(2);

/// The first 16-bit chunk of the quotient, based on little-endian ordering.
pub(crate) const COL_DIV_OUTPUT_QUOT_0: usize = super::range_check_16::col_rc_16_input(0);
/// The second 16-bit chunk of the quotient, based on little-endian ordering.
pub(crate) const COL_DIV_OUTPUT_QUOT_1: usize = super::range_check_16::col_rc_16_input(1);
/// The first 16-bit chunk of the remainder, based on little-endian ordering.
pub(crate) const COL_DIV_OUTPUT_REM_0: usize = super::range_check_16::col_rc_16_input(2);
/// The second 16-bit chunk of the remainder, based on little-endian ordering.
pub(crate) const COL_DIV_OUTPUT_REM_1: usize = super::range_check_16::col_rc_16_input(3);

/// The first 16-bit chunk of a temporary value (divisor - remainder - 1).
pub(crate) const COL_DIV_DIVISOR_REM_DIFF_M1_0: usize = super::range_check_16::col_rc_16_input(4);
/// The second 16-bit chunk of a temporary value (divisor - remainder - 1).
pub(crate) const COL_DIV_DIVISOR_REM_DIFF_M1_1: usize = super::range_check_16::col_rc_16_input(5);

pub(super) const END: usize = super::START_ALU + NUM_SHARED_COLS;
