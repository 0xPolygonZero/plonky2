//! Memory unit.

const NUM_MEMORY_OPS: usize = 4;
const NUM_MEMORY_VALUE_LIMBS: usize = 8;

pub(crate) const TIMESTAMP: usize = 0;
pub(crate) const IS_READ: usize = TIMESTAMP + 1;
pub(crate) const ADDR_CONTEXT: usize = IS_READ + 1;
pub(crate) const ADDR_SEGMENT: usize = ADDR_CONTEXT + 1;
pub(crate) const ADDR_VIRTUAL: usize = ADDR_SEGMENT + 1;

const VALUE_START: usize = ADDR_VIRTUAL + 1;
pub(crate) const fn value_limb(i: usize) -> usize {
    debug_assert!(i < NUM_MEMORY_VALUE_LIMBS);
    VALUE_START + i
}

pub(crate) const SORTED_TIMESTAMP: usize = VALUE_START + NUM_MEMORY_VALUE_LIMBS;
pub(crate) const SORTED_IS_READ: usize = SORTED_TIMESTAMP + 1;
pub(crate) const SORTED_ADDR_CONTEXT: usize = SORTED_IS_READ + 1;
pub(crate) const SORTED_ADDR_SEGMENT: usize = SORTED_ADDR_CONTEXT + 1;
pub(crate) const SORTED_ADDR_VIRTUAL: usize = SORTED_ADDR_SEGMENT + 1;

const SORTED_VALUE_START: usize = SORTED_ADDR_VIRTUAL + 1;
pub(crate) const fn sorted_value_limb(i: usize) -> usize {
    debug_assert!(i < NUM_MEMORY_VALUE_LIMBS);
    SORTED_VALUE_START + i
}

pub(crate) const CONTEXT_FIRST_CHANGE: usize = SORTED_VALUE_START + NUM_MEMORY_VALUE_LIMBS;
pub(crate) const SEGMENT_FIRST_CHANGE: usize = CONTEXT_FIRST_CHANGE + 1;
pub(crate) const VIRTUAL_FIRST_CHANGE: usize = SEGMENT_FIRST_CHANGE + 1;

pub(crate) const RANGE_CHECK: usize = VIRTUAL_FIRST_CHANGE + 1;
pub(crate) const COUNTER: usize = RANGE_CHECK + 1;
pub(crate) const RANGE_CHECK_PERMUTED: usize = COUNTER + 1;
pub(crate) const COUNTER_PERMUTED: usize = RANGE_CHECK_PERMUTED + 1;

const IS_MEMOP_START: usize = COUNTER_PERMUTED + 1;
pub(crate) const fn is_memop(i: usize) -> usize {
    debug_assert!(i < NUM_MEMORY_OPS);
    IS_MEMOP_START + i
}

pub(crate) const NUM_REGISTERS: usize = IS_MEMOP_START + NUM_MEMORY_OPS;
