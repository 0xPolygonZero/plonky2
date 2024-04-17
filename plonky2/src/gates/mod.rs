// Gates have `new` methods that return `GateRef`s.

pub mod arithmetic_base;
pub mod arithmetic_extension;
pub mod base_sum;
pub mod constant;
pub mod coset_interpolation;
pub mod exponentiation;
pub mod gate;
pub mod lookup;
pub mod lookup_table;
pub mod multiplication_extension;
pub mod noop;
pub mod packed_util;
pub mod poseidon;
pub mod poseidon_mds;
pub mod public_input;
pub mod random_access;
pub mod reducing;
pub mod reducing_extension;
pub(crate) mod selectors;
pub mod util;

// Can't use #[cfg(test)] here because it needs to be visible to other crates.
// See https://github.com/rust-lang/cargo/issues/8379
#[cfg(any(feature = "gate_testing", test))]
pub mod gate_testing;
