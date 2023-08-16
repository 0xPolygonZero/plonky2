use std::borrow::{Borrow, BorrowMut};
use std::mem::{size_of, transmute};
use std::ops::{Deref, DerefMut};

use crate::util::{indices_arr, transmute_no_compile_time_size_checks};

#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct OpsColumnsView<T: Copy> {
    // TODO: combine ADD, MUL, SUB, DIV, MOD, ADDFP254, MULFP254, SUBFP254, LT, and GT into one flag
    pub add: T,
    pub mul: T,
    pub sub: T,
    pub div: T,
    pub mod_: T,
    // TODO: combine ADDMOD, MULMOD and SUBMOD into one flag
    pub addmod: T,
    pub mulmod: T,
    pub addfp254: T,
    pub mulfp254: T,
    pub subfp254: T,
    pub submod: T,
    pub lt: T,
    pub gt: T,
    pub eq_iszero: T, // Combines EQ and ISZERO flags.
    pub logic_op: T,  // Combines AND, OR and XOR flags.
    pub not: T,
    pub byte: T,
    // TODO: combine SHL and SHR into one flag
    pub shl: T,
    pub shr: T,
    pub keccak_general: T,
    pub prover_input: T,
    pub pop: T,
    // TODO: combine JUMP and JUMPI into one flag
    pub jumps: T, // Note: This column must be 0 when is_cpu_cycle = 0.
    pub pc: T,
    pub jumpdest: T,
    pub push0: T,
    pub push: T,
    pub dup: T,
    pub swap: T,
    // TODO: combine GET_CONTEXT and SET_CONTEXT into one flag
    pub get_context: T,
    pub set_context: T,
    pub exit_kernel: T,
    pub m_op_general: T,

    pub syscall: T,
    pub exception: T,
}

// `u8` is guaranteed to have a `size_of` of 1.
pub const NUM_OPS_COLUMNS: usize = size_of::<OpsColumnsView<u8>>();

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

const fn make_col_map() -> OpsColumnsView<usize> {
    let indices_arr = indices_arr::<NUM_OPS_COLUMNS>();
    unsafe { transmute::<[usize; NUM_OPS_COLUMNS], OpsColumnsView<usize>>(indices_arr) }
}

pub const COL_MAP: OpsColumnsView<usize> = make_col_map();
