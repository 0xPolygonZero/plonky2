//! plonky2 proving system.
//!
//! This module also defines the [CircuitBuilder](circuit_builder::CircuitBuilder)
//! structure, used to build custom plonky2 circuits satisfying arbitrary statements.

pub mod circuit_builder;
pub mod circuit_data;
pub mod config;
pub(crate) mod copy_constraint;
mod get_challenges;
pub(crate) mod permutation_argument;
pub mod plonk_common;
pub mod proof;
pub mod prover;
mod validate_shape;
pub(crate) mod vanishing_poly;
pub mod vars;
pub mod verifier;
