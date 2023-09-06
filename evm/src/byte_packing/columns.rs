//! Byte packing registers.

use core::ops::Range;

use crate::byte_packing::NUM_BYTES;

/// 1 if this is an actual byte packing operation, or 0 if it's a padding row.
pub(crate) const FILTER: usize = 0;
/// 1 if this is a READ operation, and 0 if this is a WRITE operation.
pub(crate) const IS_READ: usize = FILTER + 1;
/// 1 if this is the end of a sequence of bytes.
/// This is also used as filter for the CTL.
// TODO: We should be able to remove this by leveraging `SEQUENCE_LEN` and the
// byte indices for the CTL filter.
pub(crate) const SEQUENCE_END: usize = IS_READ + 1;

pub(super) const BYTES_INDICES_START: usize = SEQUENCE_END + 1;
pub(crate) const fn index_bytes(i: usize) -> usize {
    debug_assert!(i < NUM_BYTES);
    BYTES_INDICES_START + i
}

pub(crate) const BYTE_INDICES_COLS: Range<usize> =
    BYTES_INDICES_START..BYTES_INDICES_START + NUM_BYTES;

pub(crate) const ADDR_CONTEXT: usize = BYTES_INDICES_START + NUM_BYTES;
pub(crate) const ADDR_SEGMENT: usize = ADDR_CONTEXT + 1;
pub(crate) const ADDR_VIRTUAL: usize = ADDR_SEGMENT + 1;
pub(crate) const TIMESTAMP: usize = ADDR_VIRTUAL + 1;

/// The total length of this pack of bytes.
/// Expected to not be greater than 32.
pub(crate) const SEQUENCE_LEN: usize = TIMESTAMP + 1;
/// The remaining length of this pack of bytes.
/// Expected to not be greater than 32.
// TODO: We should be able to remove this by leveraging `SEQUENCE_LEN` and the
// byte indices.
pub(crate) const REMAINING_LEN: usize = SEQUENCE_LEN + 1;

// 32 byte limbs hold a total of 256 bits.
const BYTES_VALUES_START: usize = REMAINING_LEN + 1;
pub(crate) const fn value_bytes(i: usize) -> usize {
    debug_assert!(i < NUM_BYTES);
    BYTES_VALUES_START + i
}

// We need one column for the table, then two columns for every value
// that needs to be range checked in the trace (all written bytes),
// namely the permutation of the column and the permutation of the range.
// The two permutations associated to the byte in column i will be in
// columns RC_COLS[2i] and RC_COLS[2i+1].
pub(crate) const RANGE_COUNTER: usize = BYTES_VALUES_START + NUM_BYTES;
pub(crate) const NUM_RANGE_CHECK_COLS: usize = 1 + 2 * NUM_BYTES;
pub(crate) const RC_COLS: Range<usize> = RANGE_COUNTER + 1..RANGE_COUNTER + NUM_RANGE_CHECK_COLS;

pub(crate) const NUM_COLUMNS: usize = RANGE_COUNTER + NUM_RANGE_CHECK_COLS;
