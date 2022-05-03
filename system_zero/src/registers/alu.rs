//! Arithmetic and logic unit.

pub(crate) const IS_ADD: usize = super::START_ALU;
pub(crate) const IS_SUB: usize = IS_ADD + 1;
pub(crate) const IS_MUL_ADD: usize = IS_SUB + 1;
pub(crate) const IS_DIV: usize = IS_MUL_ADD + 1;
pub(crate) const IS_AND: usize = IS_DIV + 1;
pub(crate) const IS_IOR: usize = IS_AND + 1;
pub(crate) const IS_XOR: usize = IS_IOR + 1;
pub(crate) const IS_ANDNOT: usize = IS_XOR + 1;

const START_SHARED_COLS: usize = IS_ANDNOT + 1;

/// Within the ALU, there are shared columns which can be used by any arithmetic/logic
/// circuit, depending on which one is active this cycle.
// Can be increased as needed as other operations are implemented.
const NUM_SHARED_COLS: usize = 130;

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
/// Inverse of divisor, if one exists, and 0 otherwise
pub(crate) const COL_DIV_INVDIVISOR: usize = shared_col(2);
/// 1 if divisor is nonzero and 0 otherwise
pub(crate) const COL_DIV_NONZERO_DIVISOR: usize = shared_col(3);

/// The first 16-bit chunk of the quotient, based on little-endian ordering.
pub(crate) const COL_DIV_OUTPUT_QUOT_0: usize = super::range_check_16::col_rc_16_input(0);
/// The second 16-bit chunk of the quotient, based on little-endian ordering.
pub(crate) const COL_DIV_OUTPUT_QUOT_1: usize = super::range_check_16::col_rc_16_input(1);
/// The first 16-bit chunk of the remainder, based on little-endian ordering.
pub(crate) const COL_DIV_OUTPUT_REM_0: usize = super::range_check_16::col_rc_16_input(2);
/// The second 16-bit chunk of the remainder, based on little-endian ordering.
pub(crate) const COL_DIV_OUTPUT_REM_1: usize = super::range_check_16::col_rc_16_input(3);

/// The first 16-bit chunk of a temporary value (divisor - remainder - 1).
pub(crate) const COL_DIV_RANGE_CHECKED_TMP_0: usize = super::range_check_16::col_rc_16_input(4);
/// The second 16-bit chunk of a temporary value (divisor - remainder - 1).
pub(crate) const COL_DIV_RANGE_CHECKED_TMP_1: usize = super::range_check_16::col_rc_16_input(5);

///
/// Bitwise logic operations
///

/// Bit decomposition of 64-bit values, as 32-bit low and high halves.

const fn gen_bitop_32bit_input_regs(start: usize) -> [usize; 32] {
    let mut regs = [0usize; 32];
    let mut i = 0;
    while i < 32 {
        regs[i] = shared_col(start + i);
        i += 1;
    }
    regs
}

pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_BIN_REGS: [usize; 32] = gen_bitop_32bit_input_regs(0);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_BIN_REGS: [usize; 32] = gen_bitop_32bit_input_regs(32);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_BIN_REGS: [usize; 32] = gen_bitop_32bit_input_regs(64);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_BIN_REGS: [usize; 32] = gen_bitop_32bit_input_regs(96);

/// The first 32-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_BITOP_OUTPUT_0: usize = shared_col(128);
/// The second 32-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_BITOP_OUTPUT_1: usize = shared_col(129);

pub(super) const END: usize = START_SHARED_COLS + NUM_SHARED_COLS;
