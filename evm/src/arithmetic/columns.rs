//! Arithmetic unit

use std::ops::Range;

pub const LIMB_BITS: usize = 16;
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
pub const N_LIMBS: usize = n_limbs();

pub(crate) const IS_ADD: usize = 0;
pub(crate) const IS_MUL: usize = IS_ADD + 1;
pub(crate) const IS_SUB: usize = IS_MUL + 1;
pub(crate) const IS_DIV: usize = IS_SUB + 1;
pub(crate) const IS_MOD: usize = IS_DIV + 1;
pub(crate) const IS_ADDMOD: usize = IS_MOD + 1;
pub(crate) const IS_MULMOD: usize = IS_ADDMOD + 1;
// pub(crate) const IS_ADDFP254: usize = IS_ADDMOD;
// pub(crate) const IS_MULFP254: usize = IS_MULMOD;
// pub(crate) const IS_SUBFP254: usize = IS_SUBMOD;
pub(crate) const IS_SUBMOD: usize = IS_MULMOD + 1;
pub(crate) const IS_LT: usize = IS_SUBMOD + 1;
pub(crate) const IS_GT: usize = IS_LT + 1;

pub(crate) const ALL_OPERATIONS: [usize; 10] = [
    IS_ADD, IS_MUL, IS_SUB, IS_DIV, IS_MOD, IS_ADDMOD, IS_MULMOD, IS_SUBMOD, IS_LT, IS_GT,
];

pub(crate) const START_SHARED_COLS: usize = IS_GT + 1;

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

pub(crate) const GENERAL_REGISTER_0: Range<usize> = START_SHARED_COLS..START_SHARED_COLS + N_LIMBS;
pub(crate) const GENERAL_REGISTER_1: Range<usize> =
    GENERAL_REGISTER_0.end..GENERAL_REGISTER_0.end + N_LIMBS;
pub(crate) const GENERAL_REGISTER_2: Range<usize> =
    GENERAL_REGISTER_1.end..GENERAL_REGISTER_1.end + N_LIMBS;
const GENERAL_REGISTER_3: Range<usize> = GENERAL_REGISTER_2.end..GENERAL_REGISTER_2.end + N_LIMBS;
// NB: Uses first slot of the GENERAL_REGISTER_3 register.
pub(crate) const GENERAL_REGISTER_BIT: usize = GENERAL_REGISTER_3.start;

// NB: Only one of these two sets of columns will be used for a given operation
const GENERAL_REGISTER_4: Range<usize> = GENERAL_REGISTER_3.end..GENERAL_REGISTER_3.end + N_LIMBS;
const GENERAL_REGISTER_4_DBL: Range<usize> =
    GENERAL_REGISTER_3.end..GENERAL_REGISTER_3.end + 2 * N_LIMBS;

// The auxiliary input columns overlap the general input columns
// because they correspond to the values in the second row for modular
// operations.
const AUX_REGISTER_0: Range<usize> = START_SHARED_COLS..START_SHARED_COLS + N_LIMBS;
const AUX_REGISTER_1: Range<usize> = AUX_REGISTER_0.end..AUX_REGISTER_0.end + 2 * N_LIMBS;
// These auxiliary input columns are awkwardly split across two rows,
// with the first half after the general input columns and the second
// half after the auxiliary input columns.
const AUX_REGISTER_2: Range<usize> = AUX_REGISTER_1.end..AUX_REGISTER_1.end + 2 * N_LIMBS - 1;

// Each element c of {MUL,MODULAR}_AUX_REGISTER is -2^20 <= c <= 2^20;
// this value is used as an offset so that everything is positive in
// the range checks.
pub(crate) const AUX_COEFF_ABS_MAX: i64 = 1 << 20;

// MUL takes 5 * N_LIMBS = 80 columns
pub(crate) const MUL_INPUT_0: Range<usize> = GENERAL_REGISTER_0;
pub(crate) const MUL_INPUT_1: Range<usize> = GENERAL_REGISTER_1;
pub(crate) const MUL_OUTPUT: Range<usize> = GENERAL_REGISTER_2;
pub(crate) const MUL_AUX_INPUT_LO: Range<usize> = GENERAL_REGISTER_3;
pub(crate) const MUL_AUX_INPUT_HI: Range<usize> = GENERAL_REGISTER_4;

// MULMOD takes 4 * N_LIMBS + 3 * 2*N_LIMBS + N_LIMBS = 176 columns
// but split over two rows of 96 columns and 80 columns.
//
// ADDMOD, SUBMOD, MOD and DIV are currently implemented in terms of
// the general modular code, so they also take 144 columns (also split
// over two rows).
pub(crate) const MODULAR_INPUT_0: Range<usize> = GENERAL_REGISTER_0;
pub(crate) const MODULAR_INPUT_1: Range<usize> = GENERAL_REGISTER_1;
pub(crate) const MODULAR_MODULUS: Range<usize> = GENERAL_REGISTER_2;
pub(crate) const MODULAR_OUTPUT: Range<usize> = GENERAL_REGISTER_3;
pub(crate) const MODULAR_QUO_INPUT: Range<usize> = GENERAL_REGISTER_4_DBL;
pub(crate) const MODULAR_OUT_AUX_RED: Range<usize> = AUX_REGISTER_0;
// NB: Last value is not used in AUX, it is used in MOD_IS_ZERO
pub(crate) const MODULAR_MOD_IS_ZERO: usize = AUX_REGISTER_1.start;
pub(crate) const MODULAR_AUX_INPUT_LO: Range<usize> = AUX_REGISTER_1.start + 1..AUX_REGISTER_1.end;
pub(crate) const MODULAR_AUX_INPUT_HI: Range<usize> = AUX_REGISTER_2;
// Must be set to MOD_IS_ZERO for DIV operation i.e. MOD_IS_ZERO * lv[IS_DIV]
pub(crate) const MODULAR_DIV_DENOM_IS_ZERO: usize = AUX_REGISTER_2.end;

#[allow(unused)] // TODO: Will be used when hooking into the CPU
pub(crate) const DIV_NUMERATOR: Range<usize> = MODULAR_INPUT_0;
#[allow(unused)] // TODO: Will be used when hooking into the CPU
pub(crate) const DIV_DENOMINATOR: Range<usize> = MODULAR_MODULUS;
#[allow(unused)] // TODO: Will be used when hooking into the CPU
pub(crate) const DIV_OUTPUT: Range<usize> =
    MODULAR_QUO_INPUT.start..MODULAR_QUO_INPUT.start + N_LIMBS;

// Need one column for the table, then two columns for every value
// that needs to be range checked in the trace, namely the permutation
// of the column and the permutation of the range. The two
// permutations associated to column i will be in columns RC_COLS[2i]
// and RC_COLS[2i+1].
pub(crate) const NUM_RANGE_CHECK_COLS: usize = 1 + 2 * NUM_SHARED_COLS;
pub(crate) const RANGE_COUNTER: usize = START_SHARED_COLS + NUM_SHARED_COLS;
pub(crate) const RC_COLS: Range<usize> = RANGE_COUNTER + 1..RANGE_COUNTER + 1 + 2 * NUM_SHARED_COLS;

pub const NUM_ARITH_COLUMNS: usize = START_SHARED_COLS + NUM_SHARED_COLS + NUM_RANGE_CHECK_COLS;
