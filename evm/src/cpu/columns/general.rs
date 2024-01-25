use std::borrow::{Borrow, BorrowMut};
use std::fmt::{Debug, Formatter};
use std::mem::{size_of, transmute};

/// General purpose columns, which can have different meanings depending on what CTL or other
/// operation is occurring at this row.
#[derive(Clone, Copy)]
pub(crate) union CpuGeneralColumnsView<T: Copy> {
    exception: CpuExceptionView<T>,
    logic: CpuLogicView<T>,
    jumps: CpuJumpsView<T>,
    shift: CpuShiftView<T>,
    stack: CpuStackView<T>,
}

impl<T: Copy> CpuGeneralColumnsView<T> {
    /// View of the columns used for exceptions: they are the exception code bits.
    /// SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn exception(&self) -> &CpuExceptionView<T> {
        unsafe { &self.exception }
    }

    /// Mutable view of the column required for exceptions: they are the exception code bits.
    /// SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn exception_mut(&mut self) -> &mut CpuExceptionView<T> {
        unsafe { &mut self.exception }
    }

    /// View of the columns required for logic operations.
    /// SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn logic(&self) -> &CpuLogicView<T> {
        unsafe { &self.logic }
    }

    /// Mutable view of the columns required for logic operations.
    /// SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn logic_mut(&mut self) -> &mut CpuLogicView<T> {
        unsafe { &mut self.logic }
    }

    /// View of the columns required for jump operations.
    /// SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn jumps(&self) -> &CpuJumpsView<T> {
        unsafe { &self.jumps }
    }

    /// Mutable view of the columns required for jump operations.
    /// SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn jumps_mut(&mut self) -> &mut CpuJumpsView<T> {
        unsafe { &mut self.jumps }
    }

    /// View of the columns required for shift operations.
    /// SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn shift(&self) -> &CpuShiftView<T> {
        unsafe { &self.shift }
    }

    /// Mutable view of the columns required for shift operations.
    /// SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn shift_mut(&mut self) -> &mut CpuShiftView<T> {
        unsafe { &mut self.shift }
    }

    /// View of the columns required for the stack top.
    /// SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn stack(&self) -> &CpuStackView<T> {
        unsafe { &self.stack }
    }

    /// Mutable view of the columns required for the stack top.
    /// SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn stack_mut(&mut self) -> &mut CpuStackView<T> {
        unsafe { &mut self.stack }
    }
}

impl<T: Copy + PartialEq> PartialEq<Self> for CpuGeneralColumnsView<T> {
    #[allow(clippy::unconditional_recursion)] // false positive
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

/// View of the first three `CpuGeneralColumns` containing exception code bits.
#[derive(Copy, Clone)]
pub(crate) struct CpuExceptionView<T: Copy> {
    /// Exception code as little-endian bits.
    pub(crate) exc_code_bits: [T; 3],
}

/// View of the `CpuGeneralColumns` storing pseudo-inverses used to prove logic operations.
#[derive(Copy, Clone)]
pub(crate) struct CpuLogicView<T: Copy> {
    /// Pseudoinverse of `(input0 - input1)`. Used prove that they are unequal. Assumes 32-bit limbs.
    pub(crate) diff_pinv: [T; 8],
}

/// View of the first two `CpuGeneralColumns` storing a flag and a pseudoinverse used to prove jumps.
#[derive(Copy, Clone)]
pub(crate) struct CpuJumpsView<T: Copy> {
    /// A flag indicating whether a jump should occur.
    pub(crate) should_jump: T,
    /// Pseudoinverse of `cond.iter().sum()`. Used to check `should_jump`.
    pub(crate) cond_sum_pinv: T,
}

/// View of the first `CpuGeneralColumns` storing a pseudoinverse used to prove shift operations.
#[derive(Copy, Clone)]
pub(crate) struct CpuShiftView<T: Copy> {
    /// For a shift amount of displacement: [T], this is the inverse of
    /// sum(displacement[1..]) or zero if the sum is zero.
    pub(crate) high_limb_sum_inv: T,
}

/// View of the last four `CpuGeneralColumns` storing stack-related variables. The first three are used
/// for conditionally enabling and disabling channels when reading the next `stack_top`, and the fourth one
/// is used to check for stack overflow.
#[derive(Copy, Clone)]
pub(crate) struct CpuStackView<T: Copy> {
    _unused: [T; 4],
    /// Pseudoinverse of `stack_len - num_pops`.
    pub(crate) stack_inv: T,
    /// stack_inv * stack_len.
    pub(crate) stack_inv_aux: T,
    /// Used to reduce the degree of stack constraints when needed.
    pub(crate) stack_inv_aux_2: T,
    /// Pseudoinverse of `nv.stack_len - (MAX_USER_STACK_SIZE + 1)` to check for stack overflow.
    pub(crate) stack_len_bounds_aux: T,
}

/// Number of columns shared by all the views of `CpuGeneralColumnsView`.
/// `u8` is guaranteed to have a `size_of` of 1.
pub(crate) const NUM_SHARED_COLUMNS: usize = size_of::<CpuGeneralColumnsView<u8>>();
