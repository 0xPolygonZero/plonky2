//! Fast Reed-Solomon IOP (FRI) protocol.
//!
//! It provides both a native implementation and an in-circuit version
//! of the FRI verifier for recursive proof composition.

// #[cfg(not(feature = "std"))]
// use alloc::vec::Vec;
//
// use serde::Serialize;

pub const QN: usize = 3;

pub mod boil_prover;
pub mod boil_verifier;

