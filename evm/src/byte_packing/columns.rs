//! Byte packing registers.

use crate::byte_packing::{VALUE_BYTES, VALUE_LIMBS};

// Columns for memory operations, ordered by (addr, timestamp).
/// 1 if this is an actual memory operation, or 0 if it's a padding row.
pub(crate) const FILTER: usize = 0;
/// 1 if this is the end of a sequence of bytes.
/// This is also used as filter for the CTL.
pub(crate) const SEQUENCE_END: usize = FILTER + 1;

pub(super) const BYTES_INDICES_START: usize = SEQUENCE_END + 1;
pub(crate) const fn index_bytes(i: usize) -> usize {
    debug_assert!(i < VALUE_BYTES);
    BYTES_INDICES_START + i
}

pub(crate) const ADDR_CONTEXT: usize = BYTES_INDICES_START + VALUE_BYTES;
pub(crate) const ADDR_SEGMENT: usize = ADDR_CONTEXT + 1;
pub(crate) const ADDR_VIRTUAL: usize = ADDR_SEGMENT + 1;
pub(crate) const TIMESTAMP: usize = ADDR_VIRTUAL + 1;

/// The total length of this pack of bytes.
/// Expected to not be greater than 32.
pub(crate) const SEQUENCE_LEN: usize = TIMESTAMP + 1;
/// The remaining length of this pack of bytes.
/// Expected to not be greater than 32.
pub(crate) const REMAINING_LEN: usize = SEQUENCE_LEN + 1;

// 32 byte limbs hold a total of 256 bits.
const BYTES_START: usize = REMAINING_LEN + 1;
pub(crate) const fn value_bytes(i: usize) -> usize {
    debug_assert!(i < VALUE_BYTES);
    BYTES_START + i
}

// Eight 32-bit limbs hold a total of 256 bits, representing the big-endian
// encoding of the previous byte sequence.
const VALUE_START: usize = BYTES_START + VALUE_BYTES;
pub(crate) const fn value_limb(i: usize) -> usize {
    debug_assert!(i < VALUE_LIMBS);
    VALUE_START + i
}

pub(crate) const NUM_COLUMNS: usize = VALUE_START + VALUE_LIMBS;
