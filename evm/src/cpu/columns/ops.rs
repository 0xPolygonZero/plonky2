use std::borrow::{Borrow, BorrowMut};
use std::mem::{size_of, transmute};
use std::ops::{Deref, DerefMut};

use crate::util::{indices_arr, transmute_no_compile_time_size_checks};

#[repr(C)]
#[derive(Eq, PartialEq, Debug)]
pub struct OpsColumnsView<T> {
    pub stop: T,
    pub add: T,
    pub mul: T,
    pub sub: T,
    pub div: T,
    pub sdiv: T,
    pub mod_: T,
    pub smod: T,
    pub addmod: T,
    pub mulmod: T,
    pub exp: T,
    pub signextend: T,
    pub lt: T,
    pub gt: T,
    pub slt: T,
    pub sgt: T,
    pub eq: T,     // Note: This column must be 0 when is_cpu_cycle = 0.
    pub iszero: T, // Note: This column must be 0 when is_cpu_cycle = 0.
    pub and: T,
    pub or: T,
    pub xor: T,
    pub not: T,
    pub byte: T,
    pub shl: T,
    pub shr: T,
    pub sar: T,
    pub keccak256: T,
    pub keccak_general: T,
    pub address: T,
    pub balance: T,
    pub origin: T,
    pub caller: T,
    pub callvalue: T,
    pub calldataload: T,
    pub calldatasize: T,
    pub calldatacopy: T,
    pub codesize: T,
    pub codecopy: T,
    pub gasprice: T,
    pub extcodesize: T,
    pub extcodecopy: T,
    pub returndatasize: T,
    pub returndatacopy: T,
    pub extcodehash: T,
    pub blockhash: T,
    pub coinbase: T,
    pub timestamp: T,
    pub number: T,
    pub difficulty: T,
    pub gaslimit: T,
    pub chainid: T,
    pub selfbalance: T,
    pub basefee: T,
    pub prover_input: T,
    pub pop: T,
    pub mload: T,
    pub mstore: T,
    pub mstore8: T,
    pub sload: T,
    pub sstore: T,
    pub jump: T,  // Note: This column must be 0 when is_cpu_cycle = 0.
    pub jumpi: T, // Note: This column must be 0 when is_cpu_cycle = 0.
    pub pc: T,
    pub msize: T,
    pub gas: T,
    pub jumpdest: T,
    pub get_state_root: T,
    pub set_state_root: T,
    pub get_receipt_root: T,
    pub set_receipt_root: T,
    pub push: T,
    pub dup: T,
    pub swap: T,
    pub log0: T,
    pub log1: T,
    pub log2: T,
    pub log3: T,
    pub log4: T,
    // PANIC does not get a flag; it fails at the decode stage.
    pub create: T,
    pub call: T,
    pub callcode: T,
    pub return_: T,
    pub delegatecall: T,
    pub create2: T,
    pub get_context: T,
    pub set_context: T,
    pub consume_gas: T,
    pub exit_kernel: T,
    pub staticcall: T,
    pub mload_general: T,
    pub mstore_general: T,
    pub revert: T,
    pub selfdestruct: T,

    // TODO: this doesn't actually need its own flag. We can just do `1 - sum(all other flags)`.
    pub invalid: T,
}

// `u8` is guaranteed to have a `size_of` of 1.
pub const NUM_OPS_COLUMNS: usize = size_of::<OpsColumnsView<u8>>();

impl<T> From<[T; NUM_OPS_COLUMNS]> for OpsColumnsView<T> {
    fn from(value: [T; NUM_OPS_COLUMNS]) -> Self {
        unsafe { transmute_no_compile_time_size_checks(value) }
    }
}

impl<T> From<OpsColumnsView<T>> for [T; NUM_OPS_COLUMNS] {
    fn from(value: OpsColumnsView<T>) -> Self {
        unsafe { transmute_no_compile_time_size_checks(value) }
    }
}

impl<T> Borrow<OpsColumnsView<T>> for [T; NUM_OPS_COLUMNS] {
    fn borrow(&self) -> &OpsColumnsView<T> {
        unsafe { transmute(self) }
    }
}

impl<T> BorrowMut<OpsColumnsView<T>> for [T; NUM_OPS_COLUMNS] {
    fn borrow_mut(&mut self) -> &mut OpsColumnsView<T> {
        unsafe { transmute(self) }
    }
}

impl<T> Deref for OpsColumnsView<T> {
    type Target = [T; NUM_OPS_COLUMNS];
    fn deref(&self) -> &Self::Target {
        unsafe { transmute(self) }
    }
}

impl<T> DerefMut for OpsColumnsView<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { transmute(self) }
    }
}

const fn make_col_map() -> OpsColumnsView<usize> {
    let indices_arr = indices_arr::<NUM_OPS_COLUMNS>();
    unsafe { transmute::<[usize; NUM_OPS_COLUMNS], OpsColumnsView<usize>>(indices_arr) }
}

pub const COL_MAP: OpsColumnsView<usize> = make_col_map();
