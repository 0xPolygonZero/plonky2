//! Arithmetic and logic unit.

pub(crate) const IS_ADD: usize = super::START_ALU;
pub(crate) const IS_SUB: usize = IS_ADD + 1;
pub(crate) const IS_MUL_ADD: usize = IS_SUB + 1;
pub(crate) const IS_DIV: usize = IS_MUL_ADD + 1;
pub(crate) const IS_BITAND: usize = IS_DIV + 1;
pub(crate) const IS_BITIOR: usize = IS_BITAND + 1;
pub(crate) const IS_BITXOR: usize = IS_BITIOR + 1;
pub(crate) const IS_BITANDNOT: usize = IS_BITXOR + 1;

const START_SHARED_COLS: usize = IS_BITANDNOT + 1;

/// Within the ALU, there are shared columns which can be used by any arithmetic/logic
/// circuit, depending on which one is active this cycle.
// Can be increased as needed as other operations are implemented.
const NUM_SHARED_COLS: usize = 128;

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

/// Bit decomposition of 64-bit values
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_00: usize = shared_col(0);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_01: usize = shared_col(1);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_02: usize = shared_col(2);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_03: usize = shared_col(3);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_04: usize = shared_col(4);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_05: usize = shared_col(5);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_06: usize = shared_col(6);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_07: usize = shared_col(7);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_08: usize = shared_col(8);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_09: usize = shared_col(9);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_10: usize = shared_col(10);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_11: usize = shared_col(11);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_12: usize = shared_col(12);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_13: usize = shared_col(13);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_14: usize = shared_col(14);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_15: usize = shared_col(15);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_16: usize = shared_col(16);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_17: usize = shared_col(17);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_18: usize = shared_col(18);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_19: usize = shared_col(19);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_20: usize = shared_col(20);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_21: usize = shared_col(21);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_22: usize = shared_col(22);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_23: usize = shared_col(23);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_24: usize = shared_col(24);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_25: usize = shared_col(25);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_26: usize = shared_col(26);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_27: usize = shared_col(27);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_28: usize = shared_col(28);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_29: usize = shared_col(29);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_30: usize = shared_col(30);
pub(crate) const COL_BIT_DECOMP_INPUT_A_LO_31: usize = shared_col(31);

pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_00: usize = shared_col(32);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_01: usize = shared_col(33);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_02: usize = shared_col(34);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_03: usize = shared_col(35);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_04: usize = shared_col(36);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_05: usize = shared_col(37);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_06: usize = shared_col(38);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_07: usize = shared_col(39);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_08: usize = shared_col(40);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_09: usize = shared_col(41);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_10: usize = shared_col(42);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_11: usize = shared_col(43);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_12: usize = shared_col(44);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_13: usize = shared_col(45);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_14: usize = shared_col(46);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_15: usize = shared_col(47);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_16: usize = shared_col(48);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_17: usize = shared_col(49);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_18: usize = shared_col(50);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_19: usize = shared_col(51);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_20: usize = shared_col(52);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_21: usize = shared_col(53);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_22: usize = shared_col(54);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_23: usize = shared_col(55);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_24: usize = shared_col(56);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_25: usize = shared_col(57);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_26: usize = shared_col(58);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_27: usize = shared_col(59);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_28: usize = shared_col(60);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_29: usize = shared_col(61);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_30: usize = shared_col(62);
pub(crate) const COL_BIT_DECOMP_INPUT_A_HI_31: usize = shared_col(63);

pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_00: usize = shared_col(64);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_01: usize = shared_col(65);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_02: usize = shared_col(66);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_03: usize = shared_col(67);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_04: usize = shared_col(68);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_05: usize = shared_col(69);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_06: usize = shared_col(70);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_07: usize = shared_col(71);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_08: usize = shared_col(72);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_09: usize = shared_col(73);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_10: usize = shared_col(74);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_11: usize = shared_col(75);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_12: usize = shared_col(76);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_13: usize = shared_col(77);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_14: usize = shared_col(78);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_15: usize = shared_col(79);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_16: usize = shared_col(80);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_17: usize = shared_col(81);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_18: usize = shared_col(82);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_19: usize = shared_col(83);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_20: usize = shared_col(84);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_21: usize = shared_col(85);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_22: usize = shared_col(86);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_23: usize = shared_col(87);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_24: usize = shared_col(88);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_25: usize = shared_col(89);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_26: usize = shared_col(90);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_27: usize = shared_col(91);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_28: usize = shared_col(92);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_29: usize = shared_col(93);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_30: usize = shared_col(94);
pub(crate) const COL_BIT_DECOMP_INPUT_B_LO_31: usize = shared_col(95);

pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_00: usize = shared_col(96);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_01: usize = shared_col(97);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_02: usize = shared_col(98);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_03: usize = shared_col(99);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_04: usize = shared_col(100);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_05: usize = shared_col(101);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_06: usize = shared_col(102);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_07: usize = shared_col(103);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_08: usize = shared_col(104);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_09: usize = shared_col(105);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_10: usize = shared_col(106);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_11: usize = shared_col(107);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_12: usize = shared_col(108);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_13: usize = shared_col(109);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_14: usize = shared_col(110);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_15: usize = shared_col(111);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_16: usize = shared_col(112);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_17: usize = shared_col(113);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_18: usize = shared_col(114);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_19: usize = shared_col(115);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_20: usize = shared_col(116);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_21: usize = shared_col(117);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_22: usize = shared_col(118);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_23: usize = shared_col(119);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_24: usize = shared_col(120);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_25: usize = shared_col(121);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_26: usize = shared_col(122);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_27: usize = shared_col(123);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_28: usize = shared_col(124);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_29: usize = shared_col(125);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_30: usize = shared_col(126);
pub(crate) const COL_BIT_DECOMP_INPUT_B_HI_31: usize = shared_col(127);

/// The first 16-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_BITOP_OUTPUT_0: usize = super::range_check_16::col_rc_16_input(0);
/// The second 16-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_BITOP_OUTPUT_1: usize = super::range_check_16::col_rc_16_input(1);
/// The third 16-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_BITOP_OUTPUT_2: usize = super::range_check_16::col_rc_16_input(2);
/// The fourth 16-bit chunk of the output, based on little-endian ordering.
pub(crate) const COL_BITOP_OUTPUT_3: usize = super::range_check_16::col_rc_16_input(3);

pub(super) const END: usize = START_SHARED_COLS + NUM_SHARED_COLS;
