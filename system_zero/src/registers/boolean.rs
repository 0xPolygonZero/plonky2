//! Boolean unit. Contains columns whose values must be 0 or 1.

const NUM_BITS: usize = 128;

pub const fn col_bit(index: usize) -> usize {
    debug_assert!(index < NUM_BITS);
    super::START_BOOLEAN + index
}

pub(super) const END: usize = super::START_BOOLEAN + NUM_BITS;
