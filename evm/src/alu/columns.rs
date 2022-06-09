//! Arithmetic and logic unit

pub const EVM_REGISTER_BITS: usize = 256;
pub const N_LIMBS_32: usize = EVM_REGISTER_BITS / 32;
pub const N_LIMBS_16: usize = EVM_REGISTER_BITS / 16;

pub const IS_ADD: usize = 0;
pub const IS_MUL: usize = IS_ADD + 1;
pub const IS_SUB: usize = IS_MUL + 1;
pub const IS_DIV: usize = IS_SUB + 1;
pub const IS_SDIV: usize = IS_DIV + 1;
pub const IS_MOD: usize = IS_SDIV + 1;
pub const IS_SMOD: usize = IS_MOD + 1;
pub const IS_ADDMOD: usize = IS_SMOD + 1;
pub const IS_MULMOD: usize = IS_ADDMOD + 1;
pub const IS_EXP: usize = IS_MULMOD + 1;
pub const IS_SIGNEXTEND: usize = IS_EXP + 1;
pub const IS_LT: usize = IS_SIGNEXTEND + 1;
pub const IS_GT: usize = IS_LT + 1;
pub const IS_SLT: usize = IS_GT + 1;
pub const IS_SGT: usize = IS_SLT + 1;
//pub const IS_EQ: usize = IS_SGT + 1;     // Done on CPU
//pub const IS_ISZERO: usize = IS_EQ + 1;  // Done on CPU
pub const IS_AND: usize = IS_SGT + 1;
pub const IS_OR: usize = IS_AND + 1;
pub const IS_XOR: usize = IS_OR + 1;
//pub const IS_NOT: usize = IS_XOR + 1;    // Done on CPU
pub const IS_BYTE: usize = IS_XOR + 1;
pub const IS_SHL: usize = IS_BYTE + 1;
pub const IS_SHR: usize = IS_SHL + 1;
pub const IS_SAR: usize = IS_SHR + 1;

const START_SHARED_COLS: usize = IS_SAR + 1;

/// Within the ALU, there are shared columns which can be used by any
/// arithmetic/logic circuit, depending on which one is active this cycle.
/// Can be increased as needed as other operations are implemented.
const NUM_SHARED_COLS: usize = 64;

const fn shared_col(i: usize) -> usize {
    debug_assert!(i < NUM_SHARED_COLS);
    START_SHARED_COLS + i
}

const fn gen_input_regs<const N: usize>(start: usize) -> [usize; N] {
    let mut regs = [0usize; N];
    let mut i = 0;
    while i < N {
        regs[i] = shared_col(start + i);
        i += 1;
    }
    regs
}

// Note: Addition outputs 16-bit limbs, and since these values need to
// be range-checked, we might as well use the range check unit's
// columns as our addition outputs. So the columns defined here are
// basically aliases, not columns owned by the ALU.
//
// FIXME: I have no idea if this is the right thing to do.
const fn gen_rc_output_regs<const N: usize>(start: usize) -> [usize; N] {
    let mut regs = [0usize; N];
    let mut i = 0;
    while i < N {
        // FIXME: This doesn't work
        //regs[i] = super::range_check_16::col_rc_16_input(start + i);

        // FIXME: This will override the input columns!
        regs[i] = shared_col(start + i);
        i += 1;
    }
    regs
}

pub(crate) const ADD_INPUT_0: [usize; N_LIMBS_32] = gen_input_regs::<N_LIMBS_32>(0);
pub(crate) const ADD_INPUT_1: [usize; N_LIMBS_32] = gen_input_regs::<N_LIMBS_32>(N_LIMBS_32);
pub(crate) const ADD_OUTPUT: [usize; N_LIMBS_16] = gen_rc_output_regs::<N_LIMBS_16>(0);

// TODO: Rather than repeating these for every binary operation,
// perhaps we should just declare them once and reuse?
pub(crate) const SUB_INPUT_0: [usize; N_LIMBS_32] = gen_input_regs::<N_LIMBS_32>(0);
pub(crate) const SUB_INPUT_1: [usize; N_LIMBS_32] = gen_input_regs::<N_LIMBS_32>(N_LIMBS_32);
pub(crate) const SUB_OUTPUT: [usize; N_LIMBS_16] = gen_rc_output_regs::<N_LIMBS_16>(0);

pub(crate) const MUL_INPUT_0: [usize; N_LIMBS_16] = gen_input_regs::<N_LIMBS_16>(0);
pub(crate) const MUL_INPUT_1: [usize; N_LIMBS_16] = gen_input_regs::<N_LIMBS_16>(N_LIMBS_16);
pub(crate) const MUL_AUX_INPUT: [usize; N_LIMBS_16] = gen_input_regs::<N_LIMBS_16>(2 * N_LIMBS_16);
pub(crate) const MUL_OUTPUT: [usize; N_LIMBS_16] = gen_rc_output_regs::<N_LIMBS_16>(0);

pub const NUM_ALU_COLUMNS: usize = START_SHARED_COLS + NUM_SHARED_COLS;
