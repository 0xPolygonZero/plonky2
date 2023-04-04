#![feature(generic_const_exprs)]
#![allow(clippy::needless_range_loop)]

//! This crate provides an implementation of the Poseidon2 hash function as described in
//! <https://eprint.iacr.org/2023/323.pdf> that can be seamlessly employed in Plonky2 proving
//! system. All the necessary traits and data structures necessary for Plonky2 are already
//! implemented.
//!
//! Furthermore, this crate provides a Plonky2 gate and the necessary traits and data structures to
//! employ this novel hash function in Plonky2 circuits
extern crate alloc;

pub mod poseidon2_gate;
pub mod poseidon2_goldilock;
pub mod poseidon2_hash;
