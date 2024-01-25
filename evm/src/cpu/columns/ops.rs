use std::borrow::{Borrow, BorrowMut};
use std::mem::{size_of, transmute};
use std::ops::{Deref, DerefMut};

use crate::util::transmute_no_compile_time_size_checks;

/// Structure representing the flags for the various opcodes.
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub(crate) struct OpsColumnsView<T: Copy> {
    /// Combines ADD, MUL, SUB, DIV, MOD, LT, GT and BYTE flags.
    pub binary_op: T,
    /// Combines ADDMOD, MULMOD and SUBMOD flags.
    pub ternary_op: T,
    /// Combines ADD_FP254, MUL_FP254 and SUB_FP254 flags.
    pub fp254_op: T,
    /// Combines EQ and ISZERO flags.
    pub eq_iszero: T,
    /// Combines AND, OR and XOR flags.
    pub logic_op: T,
    /// Combines NOT and POP flags.
    pub not_pop: T,
    /// Combines SHL and SHR flags.
    pub shift: T,
    /// Combines JUMPDEST and KECCAK_GENERAL flags.
    pub jumpdest_keccak_general: T,
    /// Combines JUMP and JUMPI flags.
    pub jumps: T,
    /// Combines PUSH and PROVER_INPUT flags.
    pub push_prover_input: T,
    /// Combines DUP and SWAP flags.
    pub dup_swap: T,
    /// Combines GET_CONTEXT and SET_CONTEXT flags.
    pub context_op: T,
    /// Combines MSTORE_32BYTES and MLOAD_32BYTES.
    pub m_op_32bytes: T,
    /// Flag for EXIT_KERNEL.
    pub exit_kernel: T,
    /// Combines MSTORE_GENERAL and MLOAD_GENERAL flags.
    pub m_op_general: T,
    /// Combines PC and PUSH0
    pub pc_push0: T,

    /// Flag for syscalls.
    pub syscall: T,
    /// Flag for exceptions.
    pub exception: T,
}

/// Number of columns in Cpu Stark.
/// `u8` is guaranteed to have a `size_of` of 1.
pub(crate) const NUM_OPS_COLUMNS: usize = size_of::<OpsColumnsView<u8>>();

impl<T: Copy> From<[T; NUM_OPS_COLUMNS]> for OpsColumnsView<T> {
    fn from(value: [T; NUM_OPS_COLUMNS]) -> Self {
        unsafe { transmute_no_compile_time_size_checks(value) }
    }
}

impl<T: Copy> From<OpsColumnsView<T>> for [T; NUM_OPS_COLUMNS] {
    fn from(value: OpsColumnsView<T>) -> Self {
        unsafe { transmute_no_compile_time_size_checks(value) }
    }
}

impl<T: Copy> Borrow<OpsColumnsView<T>> for [T; NUM_OPS_COLUMNS] {
    fn borrow(&self) -> &OpsColumnsView<T> {
        unsafe { transmute(self) }
    }
}

impl<T: Copy> BorrowMut<OpsColumnsView<T>> for [T; NUM_OPS_COLUMNS] {
    fn borrow_mut(&mut self) -> &mut OpsColumnsView<T> {
        unsafe { transmute(self) }
    }
}

impl<T: Copy> Deref for OpsColumnsView<T> {
    type Target = [T; NUM_OPS_COLUMNS];
    fn deref(&self) -> &Self::Target {
        unsafe { transmute(self) }
    }
}

impl<T: Copy> DerefMut for OpsColumnsView<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { transmute(self) }
    }
}
