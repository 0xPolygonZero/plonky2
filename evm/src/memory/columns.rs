//! Memory registers.

use crate::memory::{NUM_CHANNELS, VALUE_LIMBS};

// Columns for memory operations, ordered by (addr, timestamp).
pub(crate) const TIMESTAMP: usize = 0;
pub(crate) const IS_READ: usize = TIMESTAMP + 1;
pub(crate) const ADDR_CONTEXT: usize = IS_READ + 1;
pub(crate) const ADDR_SEGMENT: usize = ADDR_CONTEXT + 1;
pub(crate) const ADDR_VIRTUAL: usize = ADDR_SEGMENT + 1;

// Eight limbs to hold up to a 256-bit value.
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

// Flags to indicate if this operation came from the `i`th channel of the memory bus.
const IS_CHANNEL_START: usize = VIRTUAL_FIRST_CHANGE + 1;
pub(crate) const fn is_channel(channel: usize) -> usize {
    debug_assert!(channel < NUM_CHANNELS);
    IS_CHANNEL_START + channel
}

// We use a range check to enforce the ordering.
pub(crate) const RANGE_CHECK: usize = IS_CHANNEL_START + NUM_CHANNELS;
// The counter column (used for the range check) starts from 0 and increments.
pub(crate) const COUNTER: usize = RANGE_CHECK + 1;
// Helper columns for the permutation argument used to enforce the range check.
pub(crate) const RANGE_CHECK_PERMUTED: usize = COUNTER + 1;
pub(crate) const COUNTER_PERMUTED: usize = RANGE_CHECK_PERMUTED + 1;

pub(crate) const NUM_COLUMNS: usize = COUNTER_PERMUTED + 1;
