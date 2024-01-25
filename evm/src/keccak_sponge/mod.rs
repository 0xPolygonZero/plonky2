//! The Keccak sponge STARK is used to hash a variable amount of data which is read from memory.
//! It connects to the memory STARK to read input data, and to the Keccak-f STARK to evaluate the
//! permutation at each absorption step.

pub mod columns;
pub mod keccak_sponge_stark;
