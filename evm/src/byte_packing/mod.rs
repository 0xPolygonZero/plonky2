//! Byte packing / unpacking unit for the EVM.
//!
//! This module handles reading / writing to memory byte sequences of
//! length at most 32 in Big-Endian ordering.

pub mod byte_packing_stark;
pub mod columns;

/// Maximum number of bytes being processed by a byte (un)packing operation.
pub(crate) const NUM_BYTES: usize = 32;
