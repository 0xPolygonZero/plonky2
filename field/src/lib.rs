#![allow(incomplete_features)]
#![allow(clippy::new_without_default)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::return_self_not_must_use)]
#![feature(generic_const_exprs)]
#![feature(specialization)]
#![feature(stdsimd)]

pub(crate) mod arch;
pub mod batch_util;
pub mod cosets;
pub mod extension_field;
pub mod fft;
pub mod field_types;
pub mod goldilocks_field;
pub mod interpolation;
mod inversion;
pub mod ops;
pub mod packable;
pub mod packed_field;
pub mod polynomial;
pub mod secp256k1_base;
pub mod secp256k1_scalar;
pub mod zero_poly_coset;

#[cfg(test)]
mod field_testing;
#[cfg(test)]
mod prime_field_testing;
