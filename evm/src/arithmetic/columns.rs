//! Arithmetic unit

use std::ops::Range;

pub(crate) const LIMB_BITS: usize = 16;
const EVM_REGISTER_BITS: usize = 256;

/// Return the number of LIMB_BITS limbs that are in an EVM
/// register-sized number, panicking if LIMB_BITS doesn't divide in
/// the EVM register size.
const fn n_limbs() -> usize {
    if EVM_REGISTER_BITS % LIMB_BITS != 0 {
        panic!("limb size must divide EVM register size");
    }
    let n = EVM_REGISTER_BITS / LIMB_BITS;
    if n % 2 == 1 {
        panic!("number of limbs must be even");
    }
    n
}

/// Number of LIMB_BITS limbs that are in on EVM register-sized number.
pub(crate) const N_LIMBS: usize = n_limbs();

pub(crate) const IS_ADD: usize = 0;
pub(crate) const IS_MUL: usize = IS_ADD + 1;
pub(crate) const IS_SUB: usize = IS_MUL + 1;
pub(crate) const IS_DIV: usize = IS_SUB + 1;
pub(crate) const IS_MOD: usize = IS_DIV + 1;
pub(crate) const IS_ADDMOD: usize = IS_MOD + 1;
pub(crate) const IS_MULMOD: usize = IS_ADDMOD + 1;
pub(crate) const IS_ADDFP254: usize = IS_MULMOD + 1;
pub(crate) const IS_MULFP254: usize = IS_ADDFP254 + 1;
pub(crate) const IS_SUBFP254: usize = IS_MULFP254 + 1;
pub(crate) const IS_SUBMOD: usize = IS_SUBFP254 + 1;
pub(crate) const IS_LT: usize = IS_SUBMOD + 1;
pub(crate) const IS_GT: usize = IS_LT + 1;
pub(crate) const IS_BYTE: usize = IS_GT + 1;
pub(crate) const IS_SHL: usize = IS_BYTE + 1;
pub(crate) const IS_SHR: usize = IS_SHL + 1;
pub(crate) const IS_RANGE_CHECK: usize = IS_SHR + 1;
/// Column that stores the opcode if the operation is a range check.
pub(crate) const OPCODE_COL: usize = IS_RANGE_CHECK + 1;
pub(crate) const START_SHARED_COLS: usize = OPCODE_COL + 1;

pub(crate) const fn op_flags() -> Range<usize> {
    IS_ADD..IS_RANGE_CHECK + 1
}

/// Within the Arithmetic Unit, there are shared columns which can be
/// used by any arithmetic circuit, depending on which one is active
/// this cycle.
///
/// Modular arithmetic takes 11 * N_LIMBS columns which is split across
/// two rows, the first with 6 * N_LIMBS columns and the second with
/// 5 * N_LIMBS columns. (There are hence N_LIMBS "wasted columns" in
/// the second row.)
pub(crate) const NUM_SHARED_COLS: usize = 6 * N_LIMBS;
pub(crate) const SHARED_COLS: Range<usize> = START_SHARED_COLS..START_SHARED_COLS + NUM_SHARED_COLS;

pub(crate) const INPUT_REGISTER_0: Range<usize> = START_SHARED_COLS..START_SHARED_COLS + N_LIMBS;
pub(crate) const INPUT_REGISTER_1: Range<usize> =
    INPUT_REGISTER_0.end..INPUT_REGISTER_0.end + N_LIMBS;
pub(crate) const INPUT_REGISTER_2: Range<usize> =
    INPUT_REGISTER_1.end..INPUT_REGISTER_1.end + N_LIMBS;
pub(crate) const OUTPUT_REGISTER: Range<usize> =
    INPUT_REGISTER_2.end..INPUT_REGISTER_2.end + N_LIMBS;

// NB: Only one of AUX_INPUT_REGISTER_[01] or AUX_INPUT_REGISTER_DBL
// will be used for a given operation since they overlap
pub(crate) const AUX_INPUT_REGISTER_0: Range<usize> =
    OUTPUT_REGISTER.end..OUTPUT_REGISTER.end + N_LIMBS;
pub(crate) const AUX_INPUT_REGISTER_1: Range<usize> =
    AUX_INPUT_REGISTER_0.end..AUX_INPUT_REGISTER_0.end + N_LIMBS;
pub(crate) const AUX_INPUT_REGISTER_DBL: Range<usize> =
    OUTPUT_REGISTER.end..OUTPUT_REGISTER.end + 2 * N_LIMBS;

// The auxiliary input columns overlap the general input columns
// because they correspond to the values in the second row for modular
// operations.
const AUX_REGISTER_0: Range<usize> = START_SHARED_COLS..START_SHARED_COLS + N_LIMBS;
const AUX_REGISTER_1: Range<usize> = AUX_REGISTER_0.end..AUX_REGISTER_0.end + 2 * N_LIMBS;
const AUX_REGISTER_2: Range<usize> = AUX_REGISTER_1.end..AUX_REGISTER_1.end + 2 * N_LIMBS - 1;

// Each element c of {MUL,MODULAR}_AUX_REGISTER is -2^20 <= c <= 2^20;
// this value is used as an offset so that everything is positive in
// the range checks.
pub(crate) const AUX_COEFF_ABS_MAX: i64 = 1 << 20;

// MUL takes 5 * N_LIMBS = 80 columns
pub(crate) const MUL_AUX_INPUT_LO: Range<usize> = AUX_INPUT_REGISTER_0;
pub(crate) const MUL_AUX_INPUT_HI: Range<usize> = AUX_INPUT_REGISTER_1;

// MULMOD takes 4 * N_LIMBS + 3 * 2*N_LIMBS + N_LIMBS = 176 columns
// but split over two rows of 96 columns and 80 columns.
//
// ADDMOD, SUBMOD, MOD and DIV are currently implemented in terms of
// the general modular code, so they also take 144 columns (also split
// over two rows).
pub(crate) const MODULAR_INPUT_0: Range<usize> = INPUT_REGISTER_0;
pub(crate) const MODULAR_INPUT_1: Range<usize> = INPUT_REGISTER_1;
pub(crate) const MODULAR_MODULUS: Range<usize> = INPUT_REGISTER_2;
pub(crate) const MODULAR_OUTPUT: Range<usize> = OUTPUT_REGISTER;
pub(crate) const MODULAR_QUO_INPUT: Range<usize> = AUX_INPUT_REGISTER_DBL;
pub(crate) const MODULAR_OUT_AUX_RED: Range<usize> = AUX_REGISTER_0;
// NB: Last value is not used in AUX, it is used in MOD_IS_ZERO
pub(crate) const MODULAR_MOD_IS_ZERO: usize = AUX_REGISTER_1.start;
pub(crate) const MODULAR_AUX_INPUT_LO: Range<usize> = AUX_REGISTER_1.start + 1..AUX_REGISTER_1.end;
pub(crate) const MODULAR_AUX_INPUT_HI: Range<usize> = AUX_REGISTER_2;
// Must be set to MOD_IS_ZERO for DIV and SHR operations i.e. MOD_IS_ZERO * (lv[IS_DIV] + lv[IS_SHR]).
pub(crate) const MODULAR_DIV_DENOM_IS_ZERO: usize = AUX_REGISTER_2.end;

/// The counter column (used for the range check) starts from 0 and increments.
pub(crate) const RANGE_COUNTER: usize = START_SHARED_COLS + NUM_SHARED_COLS;
/// The frequencies column used in logUp.
pub(crate) const RC_FREQUENCIES: usize = RANGE_COUNTER + 1;

/// Number of columns in `ArithmeticStark`.
pub(crate) const NUM_ARITH_COLUMNS: usize = START_SHARED_COLS + NUM_SHARED_COLS + 2;
