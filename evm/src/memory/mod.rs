pub mod columns;
pub mod memory_stark;
pub mod segments;

// TODO: Move to CPU module, now that channels have been removed from the memory table.
pub(crate) const NUM_CHANNELS: usize = 4;
pub(crate) const VALUE_LIMBS: usize = 8;
