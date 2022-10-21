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
    EVM_REGISTER_BITS / LIMB_BITS
}

/// Number of LIMB_BITS limbs that are in on EVM register-sized number.
pub const N_LIMBS: usize = n_limbs();

pub const IS_ADD: usize = 0;
pub const IS_MUL: usize = IS_ADD + 1;
pub const IS_SUB: usize = IS_MUL + 1;
pub const IS_DIV: usize = IS_SUB + 1;
pub const IS_SDIV: usize = IS_DIV + 1;
pub const IS_MOD: usize = IS_SDIV + 1;
pub const IS_SMOD: usize = IS_MOD + 1;
pub const IS_ADDMOD: usize = IS_SMOD + 1;
pub const IS_SUBMOD: usize = IS_ADDMOD + 1;
pub const IS_MULMOD: usize = IS_SUBMOD + 1;
pub const IS_LT: usize = IS_MULMOD + 1;
pub const IS_GT: usize = IS_LT + 1;
pub const IS_SLT: usize = IS_GT + 1;
pub const IS_SGT: usize = IS_SLT + 1;
pub const IS_SHL: usize = IS_SGT + 1;
pub const IS_SHR: usize = IS_SHL + 1;
pub const IS_SAR: usize = IS_SHR + 1;

const START_SHARED_COLS: usize = IS_SAR + 1;

pub(crate) const ALL_OPERATIONS: [usize; 17] = [
    IS_ADD, IS_MUL, IS_SUB, IS_DIV, IS_SDIV, IS_MOD, IS_SMOD, IS_ADDMOD, IS_SUBMOD, IS_MULMOD,
    IS_LT, IS_GT, IS_SLT, IS_SGT, IS_SHL, IS_SHR, IS_SAR,
];

/// Within the Arithmetic Unit, there are shared columns which can be
/// used by any arithmetic circuit, depending on which one is active
/// this cycle.  Can be increased as needed as other operations are
/// implemented.
const NUM_SHARED_COLS: usize = 9 * N_LIMBS; // only need 64 for add, sub, and mul

const GENERAL_INPUT_0: Range<usize> = START_SHARED_COLS..START_SHARED_COLS + N_LIMBS;
const GENERAL_INPUT_1: Range<usize> = GENERAL_INPUT_0.end..GENERAL_INPUT_0.end + N_LIMBS;
const GENERAL_INPUT_2: Range<usize> = GENERAL_INPUT_1.end..GENERAL_INPUT_1.end + N_LIMBS;
const GENERAL_INPUT_3: Range<usize> = GENERAL_INPUT_2.end..GENERAL_INPUT_2.end + N_LIMBS;
const AUX_INPUT_0: Range<usize> = GENERAL_INPUT_3.end..GENERAL_INPUT_3.end + 2 * N_LIMBS;
const AUX_INPUT_1: Range<usize> = AUX_INPUT_0.end..AUX_INPUT_0.end + 2 * N_LIMBS;
const AUX_INPUT_2: Range<usize> = AUX_INPUT_1.end..AUX_INPUT_1.end + N_LIMBS;

pub(crate) const ADD_INPUT_0: Range<usize> = GENERAL_INPUT_0;
pub(crate) const ADD_INPUT_1: Range<usize> = GENERAL_INPUT_1;
pub(crate) const ADD_OUTPUT: Range<usize> = GENERAL_INPUT_2;

pub(crate) const SUB_INPUT_0: Range<usize> = GENERAL_INPUT_0;
pub(crate) const SUB_INPUT_1: Range<usize> = GENERAL_INPUT_1;
pub(crate) const SUB_OUTPUT: Range<usize> = GENERAL_INPUT_2;

pub(crate) const MUL_INPUT_0: Range<usize> = GENERAL_INPUT_0;
pub(crate) const MUL_INPUT_1: Range<usize> = GENERAL_INPUT_1;
pub(crate) const MUL_OUTPUT: Range<usize> = GENERAL_INPUT_2;
pub(crate) const MUL_AUX_INPUT: Range<usize> = GENERAL_INPUT_3;

pub(crate) const CMP_INPUT_0: Range<usize> = GENERAL_INPUT_0;
pub(crate) const CMP_INPUT_1: Range<usize> = GENERAL_INPUT_1;
pub(crate) const CMP_OUTPUT: usize = GENERAL_INPUT_2.start;
pub(crate) const CMP_AUX_INPUT: Range<usize> = GENERAL_INPUT_3;

pub(crate) const MODULAR_INPUT_0: Range<usize> = GENERAL_INPUT_0;
pub(crate) const MODULAR_INPUT_1: Range<usize> = GENERAL_INPUT_1;
pub(crate) const MODULAR_MODULUS: Range<usize> = GENERAL_INPUT_2;
pub(crate) const MODULAR_OUTPUT: Range<usize> = GENERAL_INPUT_3;
pub(crate) const MODULAR_QUO_INPUT: Range<usize> = AUX_INPUT_0;
// NB: Last value is not used in AUX, it is used in MOD_IS_ZERO
pub(crate) const MODULAR_AUX_INPUT: Range<usize> = AUX_INPUT_1;
pub(crate) const MODULAR_MOD_IS_ZERO: usize = AUX_INPUT_1.end - 1;
pub(crate) const MODULAR_OUT_AUX_RED: Range<usize> = AUX_INPUT_2;

#[allow(unused)] // TODO: Will be used when hooking into the CPU
pub(crate) const DIV_NUMERATOR: Range<usize> = MODULAR_INPUT_0;
#[allow(unused)] // TODO: Will be used when hooking into the CPU
pub(crate) const DIV_DENOMINATOR: Range<usize> = MODULAR_MODULUS;
#[allow(unused)] // TODO: Will be used when hooking into the CPU
pub(crate) const DIV_OUTPUT: Range<usize> = MODULAR_QUO_INPUT.start..MODULAR_QUO_INPUT.start + 16;

pub const NUM_ARITH_COLUMNS: usize = START_SHARED_COLS + NUM_SHARED_COLS;
