//! Arithmetic and logic unit.

pub(crate) const IS_ADD: usize = super::START_ALU;
pub(crate) const IS_SUB: usize = IS_ADD + 1;
pub(crate) const IS_MUL_ADD: usize = IS_SUB + 1;
pub(crate) const IS_DIV: usize = IS_MUL_ADD + 1;
pub(crate) const IS_BITAND: usize = IS_DIV + 1;

const START_SHARED_COLS: usize = IS_BITAND + 1;

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

/// Lo 32 bits of first input
pub(crate) const COL_BITAND_INPUT_A_LO: usize = shared_col(0);
/// Hi 32 bits of first input
pub(crate) const COL_BITAND_INPUT_A_HI: usize = shared_col(1);
/// Lo 32 bits of second input
pub(crate) const COL_BITAND_INPUT_B_LO: usize = shared_col(2);
/// Hi 32 bits of second input
pub(crate) const COL_BITAND_INPUT_B_HI: usize = shared_col(3);

/// The first 16-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_BITAND_OUTPUT_0: usize = super::range_check_16::col_rc_16_input(0);
/// The second 16-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_BITAND_OUTPUT_1: usize = super::range_check_16::col_rc_16_input(1);
/// The third 16-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_BITAND_OUTPUT_2: usize = super::range_check_16::col_rc_16_input(2);
/// The fourth 16-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_BITAND_OUTPUT_3: usize = super::range_check_16::col_rc_16_input(3);

/// Bit decomposition of 64-bit values
pub(crate) const COL_BITAND_INPUT_A_LO_00: usize = super::boolean::col_bit(0);
pub(crate) const COL_BITAND_INPUT_A_LO_01: usize = super::boolean::col_bit(1);
pub(crate) const COL_BITAND_INPUT_A_LO_02: usize = super::boolean::col_bit(2);
pub(crate) const COL_BITAND_INPUT_A_LO_03: usize = super::boolean::col_bit(3);
pub(crate) const COL_BITAND_INPUT_A_LO_04: usize = super::boolean::col_bit(4);
pub(crate) const COL_BITAND_INPUT_A_LO_05: usize = super::boolean::col_bit(5);
pub(crate) const COL_BITAND_INPUT_A_LO_06: usize = super::boolean::col_bit(6);
pub(crate) const COL_BITAND_INPUT_A_LO_07: usize = super::boolean::col_bit(7);
pub(crate) const COL_BITAND_INPUT_A_LO_08: usize = super::boolean::col_bit(8);
pub(crate) const COL_BITAND_INPUT_A_LO_09: usize = super::boolean::col_bit(9);
pub(crate) const COL_BITAND_INPUT_A_LO_10: usize = super::boolean::col_bit(10);
pub(crate) const COL_BITAND_INPUT_A_LO_11: usize = super::boolean::col_bit(11);
pub(crate) const COL_BITAND_INPUT_A_LO_12: usize = super::boolean::col_bit(12);
pub(crate) const COL_BITAND_INPUT_A_LO_13: usize = super::boolean::col_bit(13);
pub(crate) const COL_BITAND_INPUT_A_LO_14: usize = super::boolean::col_bit(14);
pub(crate) const COL_BITAND_INPUT_A_LO_15: usize = super::boolean::col_bit(15);
pub(crate) const COL_BITAND_INPUT_A_LO_16: usize = super::boolean::col_bit(16);
pub(crate) const COL_BITAND_INPUT_A_LO_17: usize = super::boolean::col_bit(17);
pub(crate) const COL_BITAND_INPUT_A_LO_18: usize = super::boolean::col_bit(18);
pub(crate) const COL_BITAND_INPUT_A_LO_19: usize = super::boolean::col_bit(19);
pub(crate) const COL_BITAND_INPUT_A_LO_20: usize = super::boolean::col_bit(20);
pub(crate) const COL_BITAND_INPUT_A_LO_21: usize = super::boolean::col_bit(21);
pub(crate) const COL_BITAND_INPUT_A_LO_22: usize = super::boolean::col_bit(22);
pub(crate) const COL_BITAND_INPUT_A_LO_23: usize = super::boolean::col_bit(23);
pub(crate) const COL_BITAND_INPUT_A_LO_24: usize = super::boolean::col_bit(24);
pub(crate) const COL_BITAND_INPUT_A_LO_25: usize = super::boolean::col_bit(25);
pub(crate) const COL_BITAND_INPUT_A_LO_26: usize = super::boolean::col_bit(26);
pub(crate) const COL_BITAND_INPUT_A_LO_27: usize = super::boolean::col_bit(27);
pub(crate) const COL_BITAND_INPUT_A_LO_28: usize = super::boolean::col_bit(28);
pub(crate) const COL_BITAND_INPUT_A_LO_29: usize = super::boolean::col_bit(29);
pub(crate) const COL_BITAND_INPUT_A_LO_30: usize = super::boolean::col_bit(30);
pub(crate) const COL_BITAND_INPUT_A_LO_31: usize = super::boolean::col_bit(31);

pub(crate) const COL_BITAND_INPUT_A_HI_00: usize = super::boolean::col_bit(32);
pub(crate) const COL_BITAND_INPUT_A_HI_01: usize = super::boolean::col_bit(33);
pub(crate) const COL_BITAND_INPUT_A_HI_02: usize = super::boolean::col_bit(34);
pub(crate) const COL_BITAND_INPUT_A_HI_03: usize = super::boolean::col_bit(35);
pub(crate) const COL_BITAND_INPUT_A_HI_04: usize = super::boolean::col_bit(36);
pub(crate) const COL_BITAND_INPUT_A_HI_05: usize = super::boolean::col_bit(37);
pub(crate) const COL_BITAND_INPUT_A_HI_06: usize = super::boolean::col_bit(38);
pub(crate) const COL_BITAND_INPUT_A_HI_07: usize = super::boolean::col_bit(39);
pub(crate) const COL_BITAND_INPUT_A_HI_08: usize = super::boolean::col_bit(40);
pub(crate) const COL_BITAND_INPUT_A_HI_09: usize = super::boolean::col_bit(41);
pub(crate) const COL_BITAND_INPUT_A_HI_10: usize = super::boolean::col_bit(42);
pub(crate) const COL_BITAND_INPUT_A_HI_11: usize = super::boolean::col_bit(43);
pub(crate) const COL_BITAND_INPUT_A_HI_12: usize = super::boolean::col_bit(44);
pub(crate) const COL_BITAND_INPUT_A_HI_13: usize = super::boolean::col_bit(45);
pub(crate) const COL_BITAND_INPUT_A_HI_14: usize = super::boolean::col_bit(46);
pub(crate) const COL_BITAND_INPUT_A_HI_15: usize = super::boolean::col_bit(47);
pub(crate) const COL_BITAND_INPUT_A_HI_16: usize = super::boolean::col_bit(48);
pub(crate) const COL_BITAND_INPUT_A_HI_17: usize = super::boolean::col_bit(49);
pub(crate) const COL_BITAND_INPUT_A_HI_18: usize = super::boolean::col_bit(50);
pub(crate) const COL_BITAND_INPUT_A_HI_19: usize = super::boolean::col_bit(51);
pub(crate) const COL_BITAND_INPUT_A_HI_20: usize = super::boolean::col_bit(52);
pub(crate) const COL_BITAND_INPUT_A_HI_21: usize = super::boolean::col_bit(53);
pub(crate) const COL_BITAND_INPUT_A_HI_22: usize = super::boolean::col_bit(54);
pub(crate) const COL_BITAND_INPUT_A_HI_23: usize = super::boolean::col_bit(55);
pub(crate) const COL_BITAND_INPUT_A_HI_24: usize = super::boolean::col_bit(56);
pub(crate) const COL_BITAND_INPUT_A_HI_25: usize = super::boolean::col_bit(57);
pub(crate) const COL_BITAND_INPUT_A_HI_26: usize = super::boolean::col_bit(58);
pub(crate) const COL_BITAND_INPUT_A_HI_27: usize = super::boolean::col_bit(59);
pub(crate) const COL_BITAND_INPUT_A_HI_28: usize = super::boolean::col_bit(60);
pub(crate) const COL_BITAND_INPUT_A_HI_29: usize = super::boolean::col_bit(61);
pub(crate) const COL_BITAND_INPUT_A_HI_30: usize = super::boolean::col_bit(62);
pub(crate) const COL_BITAND_INPUT_A_HI_31: usize = super::boolean::col_bit(63);

pub(crate) const COL_BITAND_INPUT_B_LO_00: usize = super::boolean::col_bit(64);
pub(crate) const COL_BITAND_INPUT_B_LO_01: usize = super::boolean::col_bit(65);
pub(crate) const COL_BITAND_INPUT_B_LO_02: usize = super::boolean::col_bit(66);
pub(crate) const COL_BITAND_INPUT_B_LO_03: usize = super::boolean::col_bit(67);
pub(crate) const COL_BITAND_INPUT_B_LO_04: usize = super::boolean::col_bit(68);
pub(crate) const COL_BITAND_INPUT_B_LO_05: usize = super::boolean::col_bit(69);
pub(crate) const COL_BITAND_INPUT_B_LO_06: usize = super::boolean::col_bit(70);
pub(crate) const COL_BITAND_INPUT_B_LO_07: usize = super::boolean::col_bit(71);
pub(crate) const COL_BITAND_INPUT_B_LO_08: usize = super::boolean::col_bit(72);
pub(crate) const COL_BITAND_INPUT_B_LO_09: usize = super::boolean::col_bit(73);
pub(crate) const COL_BITAND_INPUT_B_LO_10: usize = super::boolean::col_bit(74);
pub(crate) const COL_BITAND_INPUT_B_LO_11: usize = super::boolean::col_bit(75);
pub(crate) const COL_BITAND_INPUT_B_LO_12: usize = super::boolean::col_bit(76);
pub(crate) const COL_BITAND_INPUT_B_LO_13: usize = super::boolean::col_bit(77);
pub(crate) const COL_BITAND_INPUT_B_LO_14: usize = super::boolean::col_bit(78);
pub(crate) const COL_BITAND_INPUT_B_LO_15: usize = super::boolean::col_bit(79);
pub(crate) const COL_BITAND_INPUT_B_LO_16: usize = super::boolean::col_bit(80);
pub(crate) const COL_BITAND_INPUT_B_LO_17: usize = super::boolean::col_bit(81);
pub(crate) const COL_BITAND_INPUT_B_LO_18: usize = super::boolean::col_bit(82);
pub(crate) const COL_BITAND_INPUT_B_LO_19: usize = super::boolean::col_bit(83);
pub(crate) const COL_BITAND_INPUT_B_LO_20: usize = super::boolean::col_bit(84);
pub(crate) const COL_BITAND_INPUT_B_LO_21: usize = super::boolean::col_bit(85);
pub(crate) const COL_BITAND_INPUT_B_LO_22: usize = super::boolean::col_bit(86);
pub(crate) const COL_BITAND_INPUT_B_LO_23: usize = super::boolean::col_bit(87);
pub(crate) const COL_BITAND_INPUT_B_LO_24: usize = super::boolean::col_bit(88);
pub(crate) const COL_BITAND_INPUT_B_LO_25: usize = super::boolean::col_bit(89);
pub(crate) const COL_BITAND_INPUT_B_LO_26: usize = super::boolean::col_bit(90);
pub(crate) const COL_BITAND_INPUT_B_LO_27: usize = super::boolean::col_bit(91);
pub(crate) const COL_BITAND_INPUT_B_LO_28: usize = super::boolean::col_bit(92);
pub(crate) const COL_BITAND_INPUT_B_LO_29: usize = super::boolean::col_bit(93);
pub(crate) const COL_BITAND_INPUT_B_LO_30: usize = super::boolean::col_bit(94);
pub(crate) const COL_BITAND_INPUT_B_LO_31: usize = super::boolean::col_bit(95);

pub(crate) const COL_BITAND_INPUT_B_HI_00: usize = super::boolean::col_bit(96);
pub(crate) const COL_BITAND_INPUT_B_HI_01: usize = super::boolean::col_bit(97);
pub(crate) const COL_BITAND_INPUT_B_HI_02: usize = super::boolean::col_bit(98);
pub(crate) const COL_BITAND_INPUT_B_HI_03: usize = super::boolean::col_bit(99);
pub(crate) const COL_BITAND_INPUT_B_HI_04: usize = super::boolean::col_bit(100);
pub(crate) const COL_BITAND_INPUT_B_HI_05: usize = super::boolean::col_bit(101);
pub(crate) const COL_BITAND_INPUT_B_HI_06: usize = super::boolean::col_bit(102);
pub(crate) const COL_BITAND_INPUT_B_HI_07: usize = super::boolean::col_bit(103);
pub(crate) const COL_BITAND_INPUT_B_HI_08: usize = super::boolean::col_bit(104);
pub(crate) const COL_BITAND_INPUT_B_HI_09: usize = super::boolean::col_bit(105);
pub(crate) const COL_BITAND_INPUT_B_HI_10: usize = super::boolean::col_bit(106);
pub(crate) const COL_BITAND_INPUT_B_HI_11: usize = super::boolean::col_bit(107);
pub(crate) const COL_BITAND_INPUT_B_HI_12: usize = super::boolean::col_bit(108);
pub(crate) const COL_BITAND_INPUT_B_HI_13: usize = super::boolean::col_bit(109);
pub(crate) const COL_BITAND_INPUT_B_HI_14: usize = super::boolean::col_bit(110);
pub(crate) const COL_BITAND_INPUT_B_HI_15: usize = super::boolean::col_bit(111);
pub(crate) const COL_BITAND_INPUT_B_HI_16: usize = super::boolean::col_bit(112);
pub(crate) const COL_BITAND_INPUT_B_HI_17: usize = super::boolean::col_bit(113);
pub(crate) const COL_BITAND_INPUT_B_HI_18: usize = super::boolean::col_bit(114);
pub(crate) const COL_BITAND_INPUT_B_HI_19: usize = super::boolean::col_bit(115);
pub(crate) const COL_BITAND_INPUT_B_HI_20: usize = super::boolean::col_bit(116);
pub(crate) const COL_BITAND_INPUT_B_HI_21: usize = super::boolean::col_bit(117);
pub(crate) const COL_BITAND_INPUT_B_HI_22: usize = super::boolean::col_bit(118);
pub(crate) const COL_BITAND_INPUT_B_HI_23: usize = super::boolean::col_bit(119);
pub(crate) const COL_BITAND_INPUT_B_HI_24: usize = super::boolean::col_bit(120);
pub(crate) const COL_BITAND_INPUT_B_HI_25: usize = super::boolean::col_bit(121);
pub(crate) const COL_BITAND_INPUT_B_HI_26: usize = super::boolean::col_bit(122);
pub(crate) const COL_BITAND_INPUT_B_HI_27: usize = super::boolean::col_bit(123);
pub(crate) const COL_BITAND_INPUT_B_HI_28: usize = super::boolean::col_bit(124);
pub(crate) const COL_BITAND_INPUT_B_HI_29: usize = super::boolean::col_bit(125);
pub(crate) const COL_BITAND_INPUT_B_HI_30: usize = super::boolean::col_bit(126);
pub(crate) const COL_BITAND_INPUT_B_HI_31: usize = super::boolean::col_bit(127);

pub(super) const END: usize = START_SHARED_COLS + NUM_SHARED_COLS;
