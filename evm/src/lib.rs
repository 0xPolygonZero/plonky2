#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::field_reassign_with_default)]
#![allow(unused)]
#![feature(let_chains)]

pub mod all_stark;
pub mod arithmetic;
pub mod byte_packing;
pub mod config;
pub mod constraint_consumer;
pub mod cpu;
pub mod cross_table_lookup;
pub mod curve_pairings;
pub mod evaluation_frame;
pub mod extension_tower;
pub mod fixed_recursive_verifier;
pub mod generation;
mod get_challenges;
pub mod keccak;
pub mod keccak_sponge;
pub mod logic;
pub mod lookup;
pub mod memory;
pub mod proof;
pub mod prover;
pub mod recursive_verifier;
pub mod stark;
pub mod util;
pub mod vanishing_poly;
pub mod verifier;
pub mod witness;

#[cfg(test)]
mod stark_testing;

use eth_trie_utils::partial_trie::HashedPartialTrie;
// Set up Jemalloc
#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

pub type Node = eth_trie_utils::partial_trie::Node<HashedPartialTrie>;
