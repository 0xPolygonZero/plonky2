pub(crate) mod cosets;
pub mod crandall_field;
pub mod goldilocks_field;
pub mod proth_field;
pub mod extension_field;
pub mod fft;
pub mod field;
pub(crate) mod lagrange;

#[cfg(test)]
mod field_testing;

#[cfg(target_feature="avx2")]
pub mod crandall_field_vec;
#[cfg(target_feature="avx2")]
pub mod goldilocks_field_vec;

#[cfg(target_feature="avx512f")]
pub mod crandall_field_vec512;
#[cfg(target_feature="avx512f")]
pub mod goldilocks_field_vec512;
