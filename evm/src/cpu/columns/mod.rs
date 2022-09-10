// TODO: remove when possible.
#![allow(dead_code)]

use std::borrow::{Borrow, BorrowMut};
use std::fmt::Debug;
use std::mem::{size_of, transmute};
use std::ops::{Index, IndexMut};

use crate::cpu::columns::general::CpuGeneralColumnsView;
use crate::memory;
use crate::util::{indices_arr, transmute_no_compile_time_size_checks};

mod general;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryChannelView<T: Copy> {
    /// 1 if this row includes a memory operation in the `i`th channel of the memory bus, otherwise
    /// 0.
    pub used: T,
    pub is_read: T,
    pub addr_context: T,
    pub addr_segment: T,
    pub addr_virtual: T,
    pub value: [T; memory::VALUE_LIMBS],
}

#[repr(C)]
#[derive(Eq, PartialEq, Debug)]
pub struct CpuColumnsView<T: Copy> {
    /// Filter. 1 if the row is part of bootstrapping the kernel code, 0 otherwise.
    pub is_bootstrap_kernel: T,

    /// Filter. 1 if the row corresponds to a cycle of execution and 0 otherwise.
    /// Lets us re-use columns in non-cycle rows.
    pub is_cpu_cycle: T,

    /// If CPU cycle: The program counter for the current instruction.
    pub program_counter: T,

    /// If CPU cycle: The stack length.
    pub stack_len: T,

    /// If CPU cycle: A prover-provided value needed to show that the instruction does not cause the
    /// stack to underflow or overflow.
    pub stack_len_bounds_aux: T,

    /// If CPU cycle: We're in kernel (privileged) mode.
    pub is_kernel_mode: T,

    // If CPU cycle: flags for EVM instructions. PUSHn, DUPn, and SWAPn only get one flag each.
    // Invalid opcodes are split between a number of flags for practical reasons. Exactly one of
    // these flags must be 1.
    pub is_stop: T,
    pub is_add: T,
    pub is_mul: T,
    pub is_sub: T,
    pub is_div: T,
    pub is_sdiv: T,
    pub is_mod: T,
    pub is_smod: T,
    pub is_addmod: T,
    pub is_mulmod: T,
    pub is_exp: T,
    pub is_signextend: T,
    pub is_lt: T,
    pub is_gt: T,
    pub is_slt: T,
    pub is_sgt: T,
    pub is_eq: T,     // Note: This column must be 0 when is_cpu_cycle = 0.
    pub is_iszero: T, // Note: This column must be 0 when is_cpu_cycle = 0.
    pub is_and: T,
    pub is_or: T,
    pub is_xor: T,
    pub is_not: T,
    pub is_byte: T,
    pub is_shl: T,
    pub is_shr: T,
    pub is_sar: T,
    pub is_keccak256: T,
    pub is_address: T,
    pub is_balance: T,
    pub is_origin: T,
    pub is_caller: T,
    pub is_callvalue: T,
    pub is_calldataload: T,
    pub is_calldatasize: T,
    pub is_calldatacopy: T,
    pub is_codesize: T,
    pub is_codecopy: T,
    pub is_gasprice: T,
    pub is_extcodesize: T,
    pub is_extcodecopy: T,
    pub is_returndatasize: T,
    pub is_returndatacopy: T,
    pub is_extcodehash: T,
    pub is_blockhash: T,
    pub is_coinbase: T,
    pub is_timestamp: T,
    pub is_number: T,
    pub is_difficulty: T,
    pub is_gaslimit: T,
    pub is_chainid: T,
    pub is_selfbalance: T,
    pub is_basefee: T,
    pub is_prover_input: T,
    pub is_pop: T,
    pub is_mload: T,
    pub is_mstore: T,
    pub is_mstore8: T,
    pub is_sload: T,
    pub is_sstore: T,
    pub is_jump: T,  // Note: This column must be 0 when is_cpu_cycle = 0.
    pub is_jumpi: T, // Note: This column must be 0 when is_cpu_cycle = 0.
    pub is_pc: T,
    pub is_msize: T,
    pub is_gas: T,
    pub is_jumpdest: T,
    pub is_get_state_root: T,
    pub is_set_state_root: T,
    pub is_get_receipt_root: T,
    pub is_set_receipt_root: T,
    pub is_push: T,
    pub is_dup: T,
    pub is_swap: T,
    pub is_log0: T,
    pub is_log1: T,
    pub is_log2: T,
    pub is_log3: T,
    pub is_log4: T,
    // PANIC does not get a flag; it fails at the decode stage.
    pub is_create: T,
    pub is_call: T,
    pub is_callcode: T,
    pub is_return: T,
    pub is_delegatecall: T,
    pub is_create2: T,
    pub is_get_context: T,
    pub is_set_context: T,
    pub is_consume_gas: T,
    pub is_exit_kernel: T,
    pub is_staticcall: T,
    pub is_mload_general: T,
    pub is_mstore_general: T,
    pub is_revert: T,
    pub is_selfdestruct: T,

    pub is_invalid: T,

    /// If CPU cycle: the opcode, broken up into bits in little-endian order.
    pub opcode_bits: [T; 8],

    /// Filter. 1 iff a Keccak lookup is performed on this row.
    pub is_keccak: T,

    /// Filter. 1 iff a Keccak memory lookup is performed on this row.
    pub is_keccak_memory: T,

    pub(crate) general: CpuGeneralColumnsView<T>,

    pub(crate) clock: T,
    pub mem_channels: [MemoryChannelView<T>; memory::NUM_CHANNELS],
}

// `u8` is guaranteed to have a `size_of` of 1.
pub const NUM_CPU_COLUMNS: usize = size_of::<CpuColumnsView<u8>>();

impl<T: Copy> From<[T; NUM_CPU_COLUMNS]> for CpuColumnsView<T> {
    fn from(value: [T; NUM_CPU_COLUMNS]) -> Self {
        unsafe { transmute_no_compile_time_size_checks(value) }
    }
}

impl<T: Copy> From<CpuColumnsView<T>> for [T; NUM_CPU_COLUMNS] {
    fn from(value: CpuColumnsView<T>) -> Self {
        unsafe { transmute_no_compile_time_size_checks(value) }
    }
}

impl<T: Copy> Borrow<CpuColumnsView<T>> for [T; NUM_CPU_COLUMNS] {
    fn borrow(&self) -> &CpuColumnsView<T> {
        unsafe { transmute(self) }
    }
}

impl<T: Copy> BorrowMut<CpuColumnsView<T>> for [T; NUM_CPU_COLUMNS] {
    fn borrow_mut(&mut self) -> &mut CpuColumnsView<T> {
        unsafe { transmute(self) }
    }
}

impl<T: Copy> Borrow<[T; NUM_CPU_COLUMNS]> for CpuColumnsView<T> {
    fn borrow(&self) -> &[T; NUM_CPU_COLUMNS] {
        unsafe { transmute(self) }
    }
}

impl<T: Copy> BorrowMut<[T; NUM_CPU_COLUMNS]> for CpuColumnsView<T> {
    fn borrow_mut(&mut self) -> &mut [T; NUM_CPU_COLUMNS] {
        unsafe { transmute(self) }
    }
}

impl<T: Copy, I> Index<I> for CpuColumnsView<T>
where
    [T]: Index<I>,
{
    type Output = <[T] as Index<I>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        let arr: &[T; NUM_CPU_COLUMNS] = self.borrow();
        <[T] as Index<I>>::index(arr, index)
    }
}

impl<T: Copy, I> IndexMut<I> for CpuColumnsView<T>
where
    [T]: IndexMut<I>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        let arr: &mut [T; NUM_CPU_COLUMNS] = self.borrow_mut();
        <[T] as IndexMut<I>>::index_mut(arr, index)
    }
}

const fn make_col_map() -> CpuColumnsView<usize> {
    let indices_arr = indices_arr::<NUM_CPU_COLUMNS>();
    unsafe { transmute::<[usize; NUM_CPU_COLUMNS], CpuColumnsView<usize>>(indices_arr) }
}

pub const COL_MAP: CpuColumnsView<usize> = make_col_map();
