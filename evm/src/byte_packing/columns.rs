//! Byte packing registers.

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
const BYTES_START: usize = REMAINING_LEN + 1;
pub(crate) const fn value_bytes(i: usize) -> usize {
    debug_assert!(i < NUM_BYTES);
    BYTES_START + i
}

pub(crate) const NUM_COLUMNS: usize = BYTES_START + NUM_BYTES;
