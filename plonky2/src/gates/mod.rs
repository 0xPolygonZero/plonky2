// Gates have `new` methods that return `GateRef`s.
#![allow(clippy::new_ret_no_self)]

pub mod add_many_u32;
pub mod arithmetic_base;
pub mod arithmetic_extension;
pub mod arithmetic_u32;
pub mod assert_le;
pub mod base_sum;
pub mod binary_arithmetic;
pub mod binary_subtraction;
pub mod comparison;
pub mod constant;
pub mod exponentiation;
pub mod gate;
pub mod gate_tree;
pub mod gmimc;
pub mod interpolation;
pub mod low_degree_interpolation;
pub mod multiplication_extension;
pub mod noop;
mod packed_util;
pub mod poseidon;
pub(crate) mod poseidon_mds;
pub(crate) mod public_input;
pub mod random_access;
pub mod range_check_u32;
pub mod reducing;
pub mod reducing_extension;
pub mod subtraction_u32;
pub mod switch;
pub mod util;

// Can't use #[cfg(test)] here because it needs to be visible to other crates.
// See https://github.com/rust-lang/cargo/issues/8379
pub mod gate_testing;
