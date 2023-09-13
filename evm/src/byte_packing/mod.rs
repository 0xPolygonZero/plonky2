//! Byte packing / unpacking unit for the EVM.
//!
//! This module handles reading / writing to memory byte sequences of
//! length at most 32 in Big-Endian ordering.

pub mod byte_packing_stark;
pub mod columns;

pub(crate) const NUM_BYTES: usize = 32;
