// Gates have `new` methods that return `GateRef`s.
#![allow(clippy::new_ret_no_self)]

pub mod arithmetic;
pub mod arithmetic_u32;
pub mod assert_le;
pub mod base_sum;
pub mod comparison;
pub mod constant;
pub mod exponentiation;
pub mod gate;
pub mod gate_tree;
pub mod gmimc;
pub mod insertion;
pub mod interpolation;
pub mod noop;
pub mod poseidon;
pub(crate) mod poseidon_mds;
pub(crate) mod public_input;
pub mod random_access;
pub mod reducing;
pub mod subtraction_u32;
pub mod switch;

#[cfg(test)]
mod gate_testing;
