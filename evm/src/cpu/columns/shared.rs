use std::borrow::{Borrow, BorrowMut};
use std::fmt::{Debug, Formatter};
use std::mem::{size_of, transmute};

/// General purpose columns, which can have different meanings depending on what CTL or other
/// operation is occurring at this row.
pub(crate) union CpuGeneralColumnsView<T: Copy> {
    keccak: CpuKeccakView<T>,
    arithmetic: CpuArithmeticView<T>,
    logic: CpuLogicView<T>,
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

// `u8` is guaranteed to have a `size_of` of 1.
pub const NUM_SHARED_COLUMNS: usize = size_of::<CpuGeneralColumnsView<u8>>();
