//! plonky2 hashing logic for in-circuit hashing and Merkle proof verification
//! as well as specific hash functions implementation.

mod arch;
pub mod batch_merkle_tree;
pub mod hash_types;
pub mod hashing;
pub mod keccak;
pub mod merkle_proofs;
pub mod merkle_tree;
pub mod path_compression;
pub mod poseidon;
pub mod poseidon_goldilocks;
