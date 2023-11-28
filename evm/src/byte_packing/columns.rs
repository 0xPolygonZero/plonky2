//! Byte packing registers.

use core::ops::Range;

use crate::byte_packing::NUM_BYTES;

/// 1 if this is a READ operation, and 0 if this is a WRITE operation.
pub(crate) const IS_READ: usize = 0;

pub(super) const LEN_INDICES_START: usize = IS_READ + 1;
// There are `NUM_BYTES` columns used to represent the length of
// the input byte sequence for a (un)packing operation.
// index_len(i) is 1 iff the length is i+1.
pub(crate) const fn index_len(i: usize) -> usize {
    debug_assert!(i < NUM_BYTES);
    LEN_INDICES_START + i
}

// Note: Those are used to obtain the length of a sequence of bytes being processed.
pub(crate) const LEN_INDICES_COLS: Range<usize> = LEN_INDICES_START..LEN_INDICES_START + NUM_BYTES;

pub(crate) const ADDR_CONTEXT: usize = LEN_INDICES_START + NUM_BYTES;
pub(crate) const ADDR_SEGMENT: usize = ADDR_CONTEXT + 1;
pub(crate) const ADDR_VIRTUAL: usize = ADDR_SEGMENT + 1;
pub(crate) const TIMESTAMP: usize = ADDR_VIRTUAL + 1;

// 32 byte limbs hold a total of 256 bits.
const BYTES_VALUES_START: usize = TIMESTAMP + 1;
// There are `NUM_BYTES` columns used to store the values of the bytes
// that are being read/written for an (un)packing operation.
pub(crate) const fn value_bytes(i: usize) -> usize {
    debug_assert!(i < NUM_BYTES);
    BYTES_VALUES_START + i
}

/// Range of columns containing the bytes values.
pub(crate) const BYTE_VALUES_RANGE: Range<usize> =
    BYTES_VALUES_START..BYTES_VALUES_START + NUM_BYTES;

/// The counter column (used for the range check) starts from 0 and increments.
pub(crate) const RANGE_COUNTER: usize = BYTES_VALUES_START + NUM_BYTES;
/// The frequencies column used in logUp.
pub(crate) const RC_FREQUENCIES: usize = RANGE_COUNTER + 1;

/// Number of columns in `BytePackingStark`.
pub(crate) const NUM_COLUMNS: usize = RANGE_COUNTER + 2;
