// TODO: remove when possible.
#![allow(dead_code)]

use std::borrow::{Borrow, BorrowMut};
use std::fmt::Debug;
use std::mem::{size_of, transmute};
use std::ops::{Index, IndexMut};

use plonky2::field::types::Field;

use crate::cpu::columns::general::CpuGeneralColumnsView;
use crate::cpu::columns::ops::OpsColumnsView;
use crate::cpu::membus::NUM_GP_CHANNELS;
use crate::memory;
use crate::util::{indices_arr, transmute_no_compile_time_size_checks};

mod general;
pub(crate) mod ops;

pub type MemValue<T> = [T; memory::VALUE_LIMBS];

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
    pub value: MemValue<T>,
}

#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct CpuColumnsView<T: Copy> {
    /// Filter. 1 if the row is part of bootstrapping the kernel code, 0 otherwise.
    pub is_bootstrap_kernel: T,

    /// Filter. 1 if the row corresponds to a cycle of execution and 0 otherwise.
    /// Lets us re-use columns in non-cycle rows.
    pub is_cpu_cycle: T,

    /// If CPU cycle: Current context.
    // TODO: this is currently unconstrained
    pub context: T,

    /// If CPU cycle: Context for code memory channel.
    pub code_context: T,

    /// If CPU cycle: The program counter for the current instruction.
    pub program_counter: T,

    /// If CPU cycle: The stack length.
    pub stack_len: T,

    /// If CPU cycle: A prover-provided value needed to show that the instruction does not cause the
    /// stack to underflow or overflow.
    pub stack_len_bounds_aux: T,

    /// If CPU cycle: We're in kernel (privileged) mode.
    pub is_kernel_mode: T,

    /// If CPU cycle: Gas counter.
    pub gas: T,

    /// If CPU cycle: flags for EVM instructions (a few cannot be shared; see the comments in
    /// `OpsColumnsView`).
    pub op: OpsColumnsView<T>,

    /// If CPU cycle: the opcode, broken up into bits in little-endian order.
    pub opcode_bits: [T; 8],

    /// Filter. 1 iff a Keccak sponge lookup is performed on this row.
    pub is_keccak_sponge: T,

    pub(crate) general: CpuGeneralColumnsView<T>,

    pub(crate) clock: T,
    pub mem_channels: [MemoryChannelView<T>; NUM_GP_CHANNELS],
}

// `u8` is guaranteed to have a `size_of` of 1.
pub const NUM_CPU_COLUMNS: usize = size_of::<CpuColumnsView<u8>>();

impl<F: Field> Default for CpuColumnsView<F> {
    fn default() -> Self {
        Self::from([F::ZERO; NUM_CPU_COLUMNS])
    }
}

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
