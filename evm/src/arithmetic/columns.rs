//! Arithmetic unit

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
pub const IS_MULMOD: usize = IS_ADDMOD + 1;
pub const IS_LT: usize = IS_MULMOD + 1;
pub const IS_GT: usize = IS_LT + 1;
pub const IS_SLT: usize = IS_GT + 1;
pub const IS_SGT: usize = IS_SLT + 1;
pub const IS_SHL: usize = IS_SGT + 1;
pub const IS_SHR: usize = IS_SHL + 1;
pub const IS_SAR: usize = IS_SHR + 1;

const START_SHARED_COLS: usize = IS_SAR + 1;

pub(crate) const ALL_OPERATIONS: [usize; 16] = [
    IS_ADD, IS_MUL, IS_SUB, IS_DIV, IS_SDIV, IS_MOD, IS_SMOD, IS_ADDMOD, IS_MULMOD, IS_LT, IS_GT,
    IS_SLT, IS_SGT, IS_SHL, IS_SHR, IS_SAR,
];

/// Within the Arithmetic Unit, there are shared columns which can be
/// used by any arithmetic circuit, depending on which one is active
/// this cycle.  Can be increased as needed as other operations are
/// implemented.
const NUM_SHARED_COLS: usize = 144; // only need 64 for add, sub, and mul

const fn shared_col(i: usize) -> usize {
    assert!(i < NUM_SHARED_COLS);
    START_SHARED_COLS + i
}

const fn gen_input_cols<const N: usize>(start: usize) -> [usize; N] {
    let mut cols = [0usize; N];
    let mut i = 0;
    while i < N {
        cols[i] = shared_col(start + i);
        i += 1;
    }
    cols
}

const GENERAL_INPUT_0: [usize; N_LIMBS] = gen_input_cols::<N_LIMBS>(0);
const GENERAL_INPUT_1: [usize; N_LIMBS] = gen_input_cols::<N_LIMBS>(N_LIMBS);
const GENERAL_INPUT_2: [usize; N_LIMBS] = gen_input_cols::<N_LIMBS>(2 * N_LIMBS);
const GENERAL_INPUT_3: [usize; N_LIMBS] = gen_input_cols::<N_LIMBS>(3 * N_LIMBS);
const AUX_INPUT_0: [usize; 2 * N_LIMBS] = gen_input_cols::<{ 2 * N_LIMBS }>(4 * N_LIMBS);
const AUX_INPUT_1: [usize; 2 * N_LIMBS] = gen_input_cols::<{ 2 * N_LIMBS }>(6 * N_LIMBS);
const AUX_INPUT_2: [usize; N_LIMBS] = gen_input_cols::<N_LIMBS>(8 * N_LIMBS);

pub(crate) const ADD_INPUT_0: [usize; N_LIMBS] = GENERAL_INPUT_0;
pub(crate) const ADD_INPUT_1: [usize; N_LIMBS] = GENERAL_INPUT_1;
pub(crate) const ADD_OUTPUT: [usize; N_LIMBS] = GENERAL_INPUT_2;

pub(crate) const SUB_INPUT_0: [usize; N_LIMBS] = GENERAL_INPUT_0;
pub(crate) const SUB_INPUT_1: [usize; N_LIMBS] = GENERAL_INPUT_1;
pub(crate) const SUB_OUTPUT: [usize; N_LIMBS] = GENERAL_INPUT_2;

pub(crate) const MUL_INPUT_0: [usize; N_LIMBS] = GENERAL_INPUT_0;
pub(crate) const MUL_INPUT_1: [usize; N_LIMBS] = GENERAL_INPUT_1;
pub(crate) const MUL_OUTPUT: [usize; N_LIMBS] = GENERAL_INPUT_2;
pub(crate) const MUL_AUX_INPUT: [usize; N_LIMBS] = GENERAL_INPUT_3;

pub(crate) const CMP_INPUT_0: [usize; N_LIMBS] = GENERAL_INPUT_0;
pub(crate) const CMP_INPUT_1: [usize; N_LIMBS] = GENERAL_INPUT_1;
pub(crate) const CMP_OUTPUT: usize = GENERAL_INPUT_2[0];
pub(crate) const CMP_AUX_INPUT: [usize; N_LIMBS] = GENERAL_INPUT_3;

pub(crate) const MODULAR_INPUT_0: [usize; N_LIMBS] = GENERAL_INPUT_0;
pub(crate) const MODULAR_INPUT_1: [usize; N_LIMBS] = GENERAL_INPUT_1;
pub(crate) const MODULAR_MODULUS: [usize; N_LIMBS] = GENERAL_INPUT_2;
pub(crate) const MODULAR_OUTPUT: [usize; N_LIMBS] = GENERAL_INPUT_3;
pub(crate) const MODULAR_QUO_INPUT: [usize; 2 * N_LIMBS] = AUX_INPUT_0;
// NB: Last value is not used in AUX, it is used in IS_ZERO
pub(crate) const MODULAR_AUX_INPUT: [usize; 2 * N_LIMBS] = AUX_INPUT_1;
pub(crate) const MODULAR_MOD_IS_ZERO: usize = AUX_INPUT_1[2 * N_LIMBS - 1];
pub(crate) const MODULAR_OUT_AUX_RED: [usize; N_LIMBS] = AUX_INPUT_2;

pub const NUM_ARITH_COLUMNS: usize = START_SHARED_COLS + NUM_SHARED_COLS;
