//! Memory registers.

use crate::memory::VALUE_LIMBS;

// Columns for memory operations, ordered by (addr, timestamp).
/// 1 if this is an actual memory operation, or 0 if it's a padding row.
pub(crate) const FILTER: usize = 0;
/// Each memory operation is associated to a unique timestamp.
/// For a given memory operation `op_i`, its timestamp is computed as `C * N + i`
/// where `C` is the CPU clock at that time, `N` is the number of general memory channels,
/// and `i` is the index of the memory channel at which the memory operation is performed.
pub(crate) const TIMESTAMP: usize = FILTER + 1;
/// 1 if this is a read operation, 0 if it is a write one.
pub(crate) const IS_READ: usize = TIMESTAMP + 1;
/// The execution context of this address.
pub(crate) const ADDR_CONTEXT: usize = IS_READ + 1;
/// The segment section of this address.
pub(crate) const ADDR_SEGMENT: usize = ADDR_CONTEXT + 1;
/// The virtual address within the given context and segment.
pub(crate) const ADDR_VIRTUAL: usize = ADDR_SEGMENT + 1;

// Eight 32-bit limbs hold a total of 256 bits.
// If a value represents an integer, it is little-endian encoded.
const VALUE_START: usize = ADDR_VIRTUAL + 1;
pub(crate) const fn value_limb(i: usize) -> usize {
    debug_assert!(i < VALUE_LIMBS);
    VALUE_START + i
}

// Flags to indicate whether this part of the address differs from the next row,
// and the previous parts do not differ.
// That is, e.g., `SEGMENT_FIRST_CHANGE` is `F::ONE` iff `ADDR_CONTEXT` is the same in this
// row and the next, but `ADDR_SEGMENT` is not.
pub(crate) const CONTEXT_FIRST_CHANGE: usize = VALUE_START + VALUE_LIMBS;
pub(crate) const SEGMENT_FIRST_CHANGE: usize = CONTEXT_FIRST_CHANGE + 1;
pub(crate) const VIRTUAL_FIRST_CHANGE: usize = SEGMENT_FIRST_CHANGE + 1;

// Used to lower the degree of the zero-initializing constraints.
// Contains `next_segment * addr_changed * next_is_read`.
pub(crate) const INITIALIZE_AUX: usize = VIRTUAL_FIRST_CHANGE + 1;

// We use a range check to enforce the ordering.
pub(crate) const RANGE_CHECK: usize = INITIALIZE_AUX + 1;
/// The counter column (used for the range check) starts from 0 and increments.
pub(crate) const COUNTER: usize = RANGE_CHECK + 1;
/// The frequencies column used in logUp.
pub(crate) const FREQUENCIES: usize = COUNTER + 1;

pub(crate) const NUM_COLUMNS: usize = FREQUENCIES + 1;
