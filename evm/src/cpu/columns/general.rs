use std::borrow::{Borrow, BorrowMut};
use std::fmt::{Debug, Formatter};
use std::mem::{size_of, transmute};

/// General purpose columns, which can have different meanings depending on what CTL or other
/// operation is occurring at this row.
pub(crate) union CpuGeneralColumnsView<T: Copy> {
    keccak: CpuKeccakView<T>,
    arithmetic: CpuArithmeticView<T>,
    logic: CpuLogicView<T>,
    jumps: CpuJumpsView<T>,
    syscalls: CpuSyscallsView<T>,
}

impl<T: Copy> CpuGeneralColumnsView<T> {
    // SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn keccak(&self) -> &CpuKeccakView<T> {
        unsafe { &self.keccak }
    }

    // SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn keccak_mut(&mut self) -> &mut CpuKeccakView<T> {
        unsafe { &mut self.keccak }
    }

    // SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn arithmetic(&self) -> &CpuArithmeticView<T> {
        unsafe { &self.arithmetic }
    }

    // SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn arithmetic_mut(&mut self) -> &mut CpuArithmeticView<T> {
        unsafe { &mut self.arithmetic }
    }

    // SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn logic(&self) -> &CpuLogicView<T> {
        unsafe { &self.logic }
    }

    // SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn logic_mut(&mut self) -> &mut CpuLogicView<T> {
        unsafe { &mut self.logic }
    }

    // SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn jumps(&self) -> &CpuJumpsView<T> {
        unsafe { &self.jumps }
    }

    // SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn jumps_mut(&mut self) -> &mut CpuJumpsView<T> {
        unsafe { &mut self.jumps }
    }

    // SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn syscalls(&self) -> &CpuSyscallsView<T> {
        unsafe { &self.syscalls }
    }

    // SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn syscalls_mut(&mut self) -> &mut CpuSyscallsView<T> {
        unsafe { &mut self.syscalls }
    }
}

impl<T: Copy + PartialEq> PartialEq<Self> for CpuGeneralColumnsView<T> {
    fn eq(&self, other: &Self) -> bool {
        let self_arr: &[T; NUM_SHARED_COLUMNS] = self.borrow();
        let other_arr: &[T; NUM_SHARED_COLUMNS] = other.borrow();
        self_arr == other_arr
    }
}

impl<T: Copy + Eq> Eq for CpuGeneralColumnsView<T> {}

impl<T: Copy + Debug> Debug for CpuGeneralColumnsView<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let self_arr: &[T; NUM_SHARED_COLUMNS] = self.borrow();
        Debug::fmt(self_arr, f)
    }
}

impl<T: Copy> Borrow<[T; NUM_SHARED_COLUMNS]> for CpuGeneralColumnsView<T> {
    fn borrow(&self) -> &[T; NUM_SHARED_COLUMNS] {
        unsafe { transmute(self) }
    }
}

impl<T: Copy> BorrowMut<[T; NUM_SHARED_COLUMNS]> for CpuGeneralColumnsView<T> {
    fn borrow_mut(&mut self) -> &mut [T; NUM_SHARED_COLUMNS] {
        unsafe { transmute(self) }
    }
}

#[derive(Copy, Clone)]
pub(crate) struct CpuKeccakView<T: Copy> {
    pub(crate) input_limbs: [T; 50],
    pub(crate) output_limbs: [T; 50],
}

#[derive(Copy, Clone)]
pub(crate) struct CpuArithmeticView<T: Copy> {
    // TODO: Add "looking" columns for the arithmetic CTL.
    tmp: T, // temporary, to suppress errors
}

#[derive(Copy, Clone)]
pub(crate) struct CpuLogicView<T: Copy> {
    // Assuming a limb size of 16 bits. This can be changed, but it must be <= 28 bits.
    pub(crate) input0: [T; 16],
    pub(crate) input1: [T; 16],
    pub(crate) output: [T; 16],
}

#[derive(Copy, Clone)]
pub(crate) struct CpuJumpsView<T: Copy> {
    // Assuming a limb size of 32 bits.
    // The top stack value at entry (for jumps, the address; for `EXIT_KERNEL`, the address and new
    // privilege level).
    pub(crate) input0: [T; 8],
    // For `JUMPI`, the second stack value (the predicate). For `JUMP`, 1.
    pub(crate) input1: [T; 8],

    // The output is only used when we fault due to an invalid address. It contains the usual
    // context that we push to the stack on a trap. `output[0]` is the program counter at the time
    // of the trap (the address of the faulting instruction). `output[1]` is 1 if we were in kernel
    // mode at the time and 0 otherwise. `output[2]`, ..., `output[7]` are zero.
    pub(crate) output: [T; 8],

    // Inverse of `input0[1] + ... + input0[7]`, if one exists; otherwise, an arbitrary value.
    // Needed to prove that `input0` is nonzero.
    pub(crate) input0_upper_sum_inv: T,
    // 1 if `input0[1..7]` is zero; else 0.
    pub(crate) input0_upper_zero: T,

    // 1 if `input0[0]` is the address of a valid jump destination (i.e. `JUMPDEST` that is not part
    // of a `PUSH` immediate); else 0. Note that the kernel is allowed to jump anywhere it wants, so
    // this flag is computed but ignored in kernel mode.
    // NOTE: this flag only considers `input0[0]`, the low 32 bits of the 256-bit register. Even if
    // this flag is 1, `input0` will still be an invalid address if the high 224 bits are not 0.
    pub(crate) dst_valid: T, // TODO: populate this (check for JUMPDEST)
    // 1 if either `dst_valid` is 1 or we are in kernel mode; else 0. (Just a logical OR.)
    pub(crate) dst_valid_or_kernel: T,
    // 1 if `dst_valid_or_kernel` and `input0_upper_zero` are both 1; else 0. In other words, we are
    // allowed to jump to `input0[0]` because either it's a valid address or we're in kernel mode
    // (`dst_valid_or_kernel`), and also `input0[1..7]` are all 0 so `input0[0]` is in fact the
    // whole address (we're not being asked to jump to an address that would overflow).
    pub(crate) input0_jumpable: T,

    // Inverse of `input1[0] + ... + input1[7]`, if one exists; otherwise, an arbitrary value.
    // Needed to prove that `input1` is nonzero.
    pub(crate) input1_sum_inv: T,

    // Note that the below flags are mutually exclusive.
    // 1 if the JUMPI falls though (because input1 is 0); else 0.
    pub(crate) should_continue: T,
    // 1 if the JUMP/JUMPI does in fact jump to `input0`; else 0. This requires `input0` to be a
    // valid destination (`input0[0]` is a `JUMPDEST` not in an immediate or we are in kernel mode
    // and also `input0[1..7]` is 0) and `input1` to be nonzero.
    pub(crate) should_jump: T,
    // 1 if the JUMP/JUMPI faults; else 0. This happens when `input0` is not a valid destination
    // (`input0[0]` is not `JUMPDEST` that is notin an immediate while we are in user mode, or
    // `input0[1..7]` is nonzero) and `input1` is nonzero.
    pub(crate) should_trap: T,
}

#[derive(Copy, Clone)]
pub(crate) struct CpuSyscallsView<T: Copy> {
    // Assuming a limb size of 32 bits.
    // The output contains the context that is required to from the system call in `EXIT_KERNEL`.
    // `output[0]` contains the program counter at the time the system call was made (the address of
    // the syscall instruction). `output[1]` is 1 if we were in kernel mode at the time and 0
    // otherwise. `output[2]`, ..., `output[7]` are zero.
    pub(crate) output: [T; 8],
}

// `u8` is guaranteed to have a `size_of` of 1.
pub const NUM_SHARED_COLUMNS: usize = size_of::<CpuGeneralColumnsView<u8>>();
