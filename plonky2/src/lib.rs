#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_range_loop)]
#![cfg_attr(not(feature = "std"), no_std)]

pub extern crate alloc;

/// Re-export of `plonky2_field`.
#[doc(inline)]
pub use plonky2_field as field;

pub mod fri;
pub mod gadgets;
pub mod gates;
pub mod hash;
pub mod iop;
pub mod plonk;
pub mod recursion;
pub mod util;

#[cfg(test)]
mod lookup_test;
