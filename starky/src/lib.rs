//! A FRI-based STARK implementation over the Goldilocks field,
//! with support for recursive proof verification through the
//! plonky2 SNARK backend.

#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::type_complexity)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

mod get_challenges;

pub mod config;
pub mod constraint_consumer;
pub mod cross_table_lookup;
pub mod evaluation_frame;
pub mod lookup;
pub mod proof;
pub mod prover;
pub mod recursive_verifier;
pub mod stark;
pub mod stark_testing;
pub mod util;
mod vanishing_poly;
pub mod verifier;

#[cfg(test)]
pub mod fibonacci_stark;
