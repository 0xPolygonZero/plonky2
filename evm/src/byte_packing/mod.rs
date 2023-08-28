pub mod byte_packing_stark;
pub mod columns;
pub mod segments;

// TODO: Move to CPU module, now that channels have been removed from the memory table.
pub(crate) const NUM_CHANNELS: usize = crate::cpu::membus::NUM_CHANNELS;
pub(crate) const VALUE_LIMBS: usize = 8;
