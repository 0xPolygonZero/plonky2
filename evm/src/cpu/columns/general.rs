use std::borrow::{Borrow, BorrowMut};
use std::fmt::{Debug, Formatter};
use std::mem::{size_of, transmute};

/// General purpose columns, which can have different meanings depending on what CTL or other
/// operation is occurring at this row.
#[derive(Clone, Copy)]
pub(crate) union CpuGeneralColumnsView<T: Copy> {
    arithmetic: CpuArithmeticView<T>,
    logic: CpuLogicView<T>,
    jumps: CpuJumpsView<T>,
    shift: CpuShiftView<T>,
}

impl<T: Copy> CpuGeneralColumnsView<T> {
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
    pub(crate) fn shift(&self) -> &CpuShiftView<T> {
        unsafe { &self.shift }
    }

    // SAFETY: Each view is a valid interpretation of the underlying array.
    pub(crate) fn shift_mut(&mut self) -> &mut CpuShiftView<T> {
        unsafe { &mut self.shift }
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
pub(crate) struct CpuArithmeticView<T: Copy> {
    // TODO: Add "looking" columns for the arithmetic CTL.
    tmp: T, // temporary, to suppress errors
}

#[derive(Copy, Clone)]
pub(crate) struct CpuLogicView<T: Copy> {
    // Pseudoinverse of `(input0 - input1)`. Used prove that they are unequal. Assumes 32-bit limbs.
    pub(crate) diff_pinv: [T; 8],
}

#[derive(Copy, Clone)]
pub(crate) struct CpuJumpsView<T: Copy> {
    // A flag.
    pub(crate) should_jump: T,
    // Pseudoinverse of `cond.iter().sum()`. Used to check `should_jump`.
    pub(crate) cond_sum_pinv: T,
}

#[derive(Copy, Clone)]
pub(crate) struct CpuShiftView<T: Copy> {
    // For a shift amount of displacement: [T], this is the inverse of
    // sum(displacement[1..]) or zero if the sum is zero.
    pub(crate) high_limb_sum_inv: T,
}

// `u8` is guaranteed to have a `size_of` of 1.
pub const NUM_SHARED_COLUMNS: usize = size_of::<CpuGeneralColumnsView<u8>>();
