pub(crate) mod cosets;
pub mod extension_field;
pub mod fft;
pub mod field_types;
pub mod goldilocks_field;
pub(crate) mod interpolation;
mod inversion;
pub(crate) mod packable;
pub(crate) mod packed_field;
pub mod secp256k1_base;
pub mod secp256k1_scalar;

#[cfg(target_feature = "avx2")]
pub(crate) mod packed_avx2;

#[cfg(test)]
mod field_testing;
#[cfg(test)]
mod prime_field_testing;
