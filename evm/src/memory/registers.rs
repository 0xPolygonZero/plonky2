//! Memory unit.

pub(crate) const MEMORY_TIMESTAMP: usize = 0;
pub(crate) const MEMORY_IS_READ: usize = MEMORY_TIMESTAMP + 1;
pub(crate) const MEMORY_ADDR_CONTEXT: usize = MEMORY_IS_READ + 1;
pub(crate) const MEMORY_ADDR_SEGMENT: usize = MEMORY_ADDR_CONTEXT + 1;
pub(crate) const MEMORY_ADDR_VIRTUAL: usize = MEMORY_ADDR_SEGMENT + 1;
pub(crate) const MEMORY_VALUE_START: usize = MEMORY_ADDR_VIRTUAL + 1;

pub const fn memory_value_limb(i: usize) -> usize {
    MEMORY_VALUE_START + i
}

pub(crate) const SORTED_MEMORY_TIMESTAMP: usize = MEMORY_VALUE_START + 8;
pub(crate) const SORTED_MEMORY_IS_READ: usize = SORTED_MEMORY_TIMESTAMP + 1;
pub(crate) const SORTED_MEMORY_ADDR_CONTEXT: usize = SORTED_MEMORY_IS_READ + 1;
pub(crate) const SORTED_MEMORY_ADDR_SEGMENT: usize = SORTED_MEMORY_ADDR_CONTEXT + 1;
pub(crate) const SORTED_MEMORY_ADDR_VIRTUAL: usize = SORTED_MEMORY_ADDR_SEGMENT + 1;
pub(crate) const SORTED_MEMORY_VALUE_START: usize = SORTED_MEMORY_ADDR_VIRTUAL + 1;

pub const fn sorted_memory_value_limb(i: usize) -> usize {
    SORTED_MEMORY_VALUE_START + i
}

pub(crate) const MEMORY_CONTEXT_FIRST_CHANGE: usize = SORTED_MEMORY_VALUE_START + 8;
pub(crate) const MEMORY_SEGMENT_FIRST_CHANGE: usize = MEMORY_CONTEXT_FIRST_CHANGE + 1;
pub(crate) const MEMORY_VIRTUAL_FIRST_CHANGE: usize = MEMORY_SEGMENT_FIRST_CHANGE + 1;

pub(crate) const MEMORY_RANGE_CHECK: usize = MEMORY_VIRTUAL_FIRST_CHANGE + 1;
pub(crate) const MEMORY_COUNTER: usize = MEMORY_RANGE_CHECK + 1;
pub(crate) const MEMORY_RANGE_CHECK_PERMUTED: usize = MEMORY_COUNTER + 1;
pub(crate) const MEMORY_COUNTER_PERMUTED: usize = MEMORY_RANGE_CHECK_PERMUTED + 1;

pub(crate) const NUM_REGISTERS: usize = MEMORY_COUNTER_PERMUTED + 1;
