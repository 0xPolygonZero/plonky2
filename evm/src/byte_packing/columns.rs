//! Byte packing registers.

use core::ops::Range;

use crate::byte_packing::NUM_BYTES;

/// 1 if this is a READ operation, and 0 if this is a WRITE operation.
pub(crate) const IS_READ: usize = 0;
/// 1 if this is the end of a sequence of bytes.
/// This is also used as filter for the CTL.
pub(crate) const SEQUENCE_END: usize = IS_READ + 1;

pub(super) const BYTES_INDICES_START: usize = SEQUENCE_END + 1;
// There are `NUM_BYTES` columns used to represent the index of the active byte
// for a given row of a byte (un)packing operation.
pub(crate) const fn index_bytes(i: usize) -> usize {
    debug_assert!(i < NUM_BYTES);
    BYTES_INDICES_START + i
}

// Note: Those are used as filter for distinguishing active vs padding rows,
// and also to obtain the length of a sequence of bytes being processed.
pub(crate) const BYTE_INDICES_COLS: Range<usize> =
    BYTES_INDICES_START..BYTES_INDICES_START + NUM_BYTES;

pub(crate) const ADDR_CONTEXT: usize = BYTES_INDICES_START + NUM_BYTES;
pub(crate) const ADDR_SEGMENT: usize = ADDR_CONTEXT + 1;
pub(crate) const ADDR_VIRTUAL: usize = ADDR_SEGMENT + 1;
pub(crate) const TIMESTAMP: usize = ADDR_VIRTUAL + 1;

// 32 byte limbs hold a total of 256 bits.
const BYTES_VALUES_START: usize = TIMESTAMP + 1;
// There are `NUM_BYTES` columns used to store the values of the bytes
// that are being read/written for an (un)packing operation.
// If `index_bytes(i) == 1`, then all `value_bytes(j) for j <= i` may be non-zero.
pub(crate) const fn value_bytes(i: usize) -> usize {
    debug_assert!(i < NUM_BYTES);
    BYTES_VALUES_START + i
}

/// The counter column (used for the range check) starts from 0 and increments.
pub(crate) const RANGE_COUNTER: usize = BYTES_VALUES_START + NUM_BYTES;
/// The frequencies column used in logUp.
pub(crate) const RC_FREQUENCIES: usize = RANGE_COUNTER + 1;

/// Number of columns in `BytePackingStark`.
pub(crate) const NUM_COLUMNS: usize = RANGE_COUNTER + 2;
