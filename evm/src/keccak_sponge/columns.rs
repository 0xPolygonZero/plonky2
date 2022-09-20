use std::borrow::{Borrow, BorrowMut};
use std::mem::{size_of, transmute};

use crate::util::{indices_arr, transmute_no_compile_time_size_checks};

pub(crate) const KECCAK_WIDTH_BYTES: usize = 200;
pub(crate) const KECCAK_WIDTH_U32S: usize = KECCAK_WIDTH_BYTES / 4;
pub(crate) const KECCAK_RATE_BYTES: usize = 136;
pub(crate) const KECCAK_RATE_U32S: usize = KECCAK_RATE_BYTES / 4;
pub(crate) const KECCAK_CAPACITY_BYTES: usize = 64;
pub(crate) const KECCAK_CAPACITY_U32S: usize = KECCAK_CAPACITY_BYTES / 4;

#[repr(C)]
#[derive(Eq, PartialEq, Debug)]
pub(crate) struct KeccakSpongeColumnsView<T: Copy> {
    /// 1 if this row represents a full input block, i.e. one in which each byte is an input byte,
    /// not a padding byte; 0 otherwise.
    pub is_full_input_block: T,

    /// 1 if this row represents the final block of a sponge, in which case some or all of the bytes
    /// in the block will be padding bytes; 0 otherwise.
    pub is_final_block: T,

    // The address at which we will read the input block.
    pub context: T,
    pub segment: T,
    pub virt: T,

    /// The timestamp at which inputs should be read from memory.
    pub timestamp: T,

    /// The length of the original input, in bytes.
    pub len: T,

    /// The number of input bytes that have already been absorbed prior to this block.
    pub already_absorbed_bytes: T,

    /// If this row represents a final block row, the `i`th entry should be 1 if the final chunk of
    /// input has length `i` (in other words if `len - already_absorbed == i`), otherwise 0.
    ///
    /// If this row represents a full input block, this should contain all 0s.
    pub is_final_input_len: [T; KECCAK_RATE_BYTES],

    /// The initial rate part of the sponge, at the start of this step.
    pub original_rate_u32s: [T; KECCAK_RATE_U32S],

    /// The capacity part of the sponge, encoded as 32-bit chunks, at the start of this step.
    pub original_capacity_u32s: [T; KECCAK_CAPACITY_U32S],

    /// The block being absorbed, which may contain input bytes and/or padding bytes.
    pub block_bytes: [T; KECCAK_RATE_BYTES],

    /// The rate part of the sponge, encoded as 32-bit chunks, after the current block is xor'd in,
    /// but before the permutation is applied.
    pub xored_rate_u32s: [T; KECCAK_RATE_U32S],

    /// The entire state (rate + capacity) of the sponge, encoded as 32-bit chunks, after the
    /// permutation is applied.
    pub updated_state_u32s: [T; KECCAK_WIDTH_U32S],
}

// `u8` is guaranteed to have a `size_of` of 1.
pub const NUM_KECCAK_SPONGE_COLUMNS: usize = size_of::<KeccakSpongeColumnsView<u8>>();

impl<T: Copy> From<[T; NUM_KECCAK_SPONGE_COLUMNS]> for KeccakSpongeColumnsView<T> {
    fn from(value: [T; NUM_KECCAK_SPONGE_COLUMNS]) -> Self {
        unsafe { transmute_no_compile_time_size_checks(value) }
    }
}

impl<T: Copy> From<KeccakSpongeColumnsView<T>> for [T; NUM_KECCAK_SPONGE_COLUMNS] {
    fn from(value: KeccakSpongeColumnsView<T>) -> Self {
        unsafe { transmute_no_compile_time_size_checks(value) }
    }
}

impl<T: Copy> Borrow<KeccakSpongeColumnsView<T>> for [T; NUM_KECCAK_SPONGE_COLUMNS] {
    fn borrow(&self) -> &KeccakSpongeColumnsView<T> {
        unsafe { transmute(self) }
    }
}

impl<T: Copy> BorrowMut<KeccakSpongeColumnsView<T>> for [T; NUM_KECCAK_SPONGE_COLUMNS] {
    fn borrow_mut(&mut self) -> &mut KeccakSpongeColumnsView<T> {
        unsafe { transmute(self) }
    }
}

impl<T: Copy> Borrow<[T; NUM_KECCAK_SPONGE_COLUMNS]> for KeccakSpongeColumnsView<T> {
    fn borrow(&self) -> &[T; NUM_KECCAK_SPONGE_COLUMNS] {
        unsafe { transmute(self) }
    }
}

impl<T: Copy> BorrowMut<[T; NUM_KECCAK_SPONGE_COLUMNS]> for KeccakSpongeColumnsView<T> {
    fn borrow_mut(&mut self) -> &mut [T; NUM_KECCAK_SPONGE_COLUMNS] {
        unsafe { transmute(self) }
    }
}

impl<T: Copy + Default> Default for KeccakSpongeColumnsView<T> {
    fn default() -> Self {
        [T::default(); NUM_KECCAK_SPONGE_COLUMNS].into()
    }
}

const fn make_col_map() -> KeccakSpongeColumnsView<usize> {
    let indices_arr = indices_arr::<NUM_KECCAK_SPONGE_COLUMNS>();
    unsafe {
        transmute::<[usize; NUM_KECCAK_SPONGE_COLUMNS], KeccakSpongeColumnsView<usize>>(indices_arr)
    }
}

pub(crate) const KECCAK_SPONGE_COL_MAP: KeccakSpongeColumnsView<usize> = make_col_map();
