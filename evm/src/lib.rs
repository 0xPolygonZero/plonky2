#![allow(incomplete_features)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![feature(let_chains)]
#![feature(generic_const_exprs)]

pub mod all_stark;
pub mod arithmetic;
pub mod config;
pub mod constraint_consumer;
pub mod cpu;
pub mod cross_table_lookup;
pub mod generation;
mod get_challenges;
pub mod keccak;
pub mod keccak_memory;
pub mod keccak_sponge;
pub mod logic;
pub mod lookup;
pub mod memory;
pub mod permutation;
pub mod proof;
pub mod prover;
pub mod recursive_verifier;
pub mod stark;
pub mod stark_testing;
pub mod util;
pub mod vanishing_poly;
pub mod vars;
pub mod verifier;
