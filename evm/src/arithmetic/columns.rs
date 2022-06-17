//! Arithmetic unit

pub const LIMB_BITS: usize = 16;
pub const EVM_REGISTER_BITS: usize = 256;
pub const N_LIMBS: usize = EVM_REGISTER_BITS / LIMB_BITS;

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

/// Within the Arithmetic Unit, there are shared columns which can be
/// used by any arithmetic circuit, depending on which one is active
/// this cycle.  Can be increased as needed as other operations are
/// implemented.
const NUM_SHARED_COLS: usize = 64;

const fn shared_col(i: usize) -> usize {
    debug_assert!(i < NUM_SHARED_COLS);
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
const GENERAL_INPUT_2: [usize; N_LIMBS] = gen_input_cols::<N_LIMBS>(2*N_LIMBS);
const AUX_INPUT_0: [usize; N_LIMBS-1] = gen_input_cols::<{N_LIMBS-1}>(3*N_LIMBS);

pub(crate) const ADD_INPUT_0: [usize; N_LIMBS] = GENERAL_INPUT_0;
pub(crate) const ADD_INPUT_1: [usize; N_LIMBS] = GENERAL_INPUT_1;
pub(crate) const ADD_OUTPUT: [usize; N_LIMBS] = GENERAL_INPUT_2;

pub(crate) const SUB_INPUT_0: [usize; N_LIMBS] = GENERAL_INPUT_0;
pub(crate) const SUB_INPUT_1: [usize; N_LIMBS] = GENERAL_INPUT_1;
pub(crate) const SUB_OUTPUT: [usize; N_LIMBS] = GENERAL_INPUT_2;

pub(crate) const MUL_INPUT_0: [usize; N_LIMBS] = GENERAL_INPUT_0;
pub(crate) const MUL_INPUT_1: [usize; N_LIMBS] = GENERAL_INPUT_0;
pub(crate) const MUL_OUTPUT: [usize; N_LIMBS] = GENERAL_INPUT_0;
pub(crate) const MUL_AUX_INPUT: [usize; N_LIMBS-1] = AUX_INPUT_0;

pub const NUM_ARITH_COLUMNS: usize = START_SHARED_COLS + NUM_SHARED_COLS;
