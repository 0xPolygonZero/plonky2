pub mod gmimc;
pub mod hash_types;
pub mod hashing;
pub mod merkle_proofs;
pub mod merkle_tree;
pub mod path_compression;
pub mod poseidon;
pub mod poseidon_crandall;
pub mod poseidon_goldilocks;
pub mod rescue;

mod arch;

#[cfg(target_feature = "avx2")]
mod poseidon_avx2;

#[cfg(target_feature = "neon")]
mod poseidon_neon;
