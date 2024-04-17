//! An implementation of a Type 1 zk-EVM by Polygon Zero.
//!
//! Following the [zk-EVM classification of V. Buterin](https://vitalik.eth.limo/general/2022/08/04/zkevm.html),
//! the plonky2_evm crate aims at providing an efficient solution for the problem of generating cryptographic
//! proofs of Ethereum-like transactions with *full Ethereum capability*.
//!
//! To this end, the plonky2 zk-EVM is tailored for an AIR-based STARK system satisfying degree 3 constraints,
//! with support for recursive aggregation leveraging plonky2 circuits with FRI-based plonkish arithmetization.
//! These circuits require a one-time, offline preprocessing phase.
//! See the [`fixed_recursive_verifier`] module for more details on how this works.
//! These preprocessed circuits are gathered within the [`AllRecursiveCircuits`] prover state,
//! and can be generated as such:
//!
//! ```ignore
//! // Specify the base field to use.
//! type F = GoldilocksField;
//! // Specify the extension degree to use.
//! const D: usize = 2;
//! // Specify the recursive configuration to use, here leveraging Poseidon hash
//! // over the Goldilocks field both natively and in-circuit.
//! type C = PoseidonGoldilocksConfig;
//!
//! let all_stark = AllStark::<F, D>::default();
//! let config = StarkConfig::standard_fast_config();
//!
//! // Generate all the recursive circuits needed to generate succinct proofs for blocks.
//! // The ranges correspond to the supported table sizes for each individual STARK component.
//! let prover_state = AllRecursiveCircuits::<F, C, D>::new(
//!     &all_stark,
//!     &[16..25, 10..20, 12..25, 14..25, 9..20, 12..20, 17..30],
//!     &config,
//! );
//! ```
//!
//! # Inputs type
//!
//! Transactions need to be processed into an Intermediary Representation (IR) format for the prover
//! to be able to generate proofs of valid state transition. This involves passing the encoded transaction,
//! the header of the block in which it was included, some information on the state prior execution
//! of this transaction, etc.
//! This intermediary representation is called [`GenerationInputs`].
//!
//!
//! # Generating succinct proofs
//!
//! ## Transaction proofs
//!
//! To generate a proof for a transaction, given its [`GenerationInputs`] and an [`AllRecursiveCircuits`]
//! prover state, one can simply call the [prove_root](AllRecursiveCircuits::prove_root) method.
//!
//! ```ignore
//! let mut timing = TimingTree::new("prove", log::Level::Debug);
//! let kill_signal = None; // Useful only with distributed proving to kill hanging jobs.
//! let (proof, public_values) =
//!     prover_state.prove_root(all_stark, config, inputs, &mut timing, kill_signal);
//! ```
//!
//! This outputs a transaction proof and its associated public values. These are necessary during the
//! aggregation levels (see below). If one were to miss the public values, they are also retrievable directly
//! from the proof's encoded public inputs, as such:
//!
//! ```ignore
//! let public_values = PublicValues::from_public_inputs(&proof.public_inputs);
//! ```
//!
//! ## Aggregation proofs
//!
//! Because the plonky2 zkEVM generates proofs on a transaction basis, we then need to aggregate them for succinct
//! verification. This is done in a binary tree fashion, where each inner node proof verifies two children proofs,
//! through the [prove_aggregation](AllRecursiveCircuits::prove_aggregation) method.
//! Note that the tree does *not* need to be complete, as this aggregation process can take as inputs both regular
//! transaction proofs and aggregation proofs. We only need to specify for each child if it is an aggregation proof
//! or a regular one.
//!
//! ```ignore
//! let (proof_1, pv_1) =
//!     prover_state.prove_root(all_stark, config, inputs_1, &mut timing, None);
//! let (proof_2, pv_2) =
//!     prover_state.prove_root(all_stark, config, inputs_2, &mut timing, None);
//! let (proof_3, pv_3) =
//!     prover_state.prove_root(all_stark, config, inputs_3, &mut timing, None);
//!
//! // Now aggregate proofs for txn 1 and 2.
//! let (agg_proof_1_2, pv_1_2) =
//!     prover_state.prove_aggregation(false, proof_1, pv_1, false, proof_2, pv_2);
//!
//! // Now aggregate the newly generated aggregation proof with the last regular txn proof.
//! let (agg_proof_1_3, pv_1_3) =
//!     prover_state.prove_aggregation(true, agg_proof_1_2, pv_1_2, false, proof_3, pv_3);
//! ```
//!
//! **Note**: The proofs provided to the [prove_aggregation](AllRecursiveCircuits::prove_aggregation) method *MUST* have contiguous states.
//! Trying to combine `proof_1` and `proof_3` from the example above would fail.
//!
//! ## Block proofs
//!
//! Once all transactions of a block have been proven and we are left with a single aggregation proof and its public values,
//! we can then wrap it into a final block proof, attesting validity of the entire block.
//! This [prove_block](AllRecursiveCircuits::prove_block) method accepts an optional previous block proof as argument,
//! which will then try combining the previously proven block with the current one, generating a validity proof for both.
//! Applying this process from genesis would yield a single proof attesting correctness of the entire chain.
//!
//! ```ignore
//! let previous_block_proof = { ... };
//! let (block_proof, block_public_values) =
//!     prover_state.prove_block(Some(&previous_block_proof), &agg_proof, agg_pv)?;
//! ```
//!
//! ### Checkpoint heights
//!
//! The process of always providing a previous block proof when generating a proof for the current block may yield some
//! undesirable issues. For this reason, the plonky2 zk-EVM supports checkpoint heights. At given block heights,
//! the prover does not have to pass a previous block proof. This would in practice correspond to block heights at which
//! a proof has been generated and sent to L1 for settlement.
//!
//! The only requirement when generating a block proof without passing a previous one as argument is to have the
//! `checkpoint_state_trie_root` metadata in the `PublicValues` of the final aggregation proof be matching the state
//! trie before applying all the included transactions. If this condition is not met, the prover will fail to generate
//! a valid proof.
//!
//!
//! ```ignore
//! let (block_proof, block_public_values) =
//!     prover_state.prove_block(None, &agg_proof, agg_pv)?;
//! ```
//!
//! # Prover state serialization
//!
//! Because the recursive circuits only need to be generated once, they can be saved to disk once the preprocessing phase
//! completed successfully, and deserialized on-demand.
//! The plonky2 zk-EVM provides serialization methods to convert the entire prover state to a vector of bytes, and vice-versa.
//! This requires the use of custom serializers for gates and generators for proper recursive circuit encoding. This crate provides
//! default serializers supporting all custom gates and associated generators defined within the [`plonky2`] crate.
//!
//! ```ignore
//! let prover_state = AllRecursiveCircuits::<F, C, D>::new(...);
//!
//! // Default serializers
//! let gate_serializer = DefaultGateSerializer;
//! let generator_serializer = DefaultGeneratorSerializer::<C, D> {
//!     _phantom: PhantomData::<C>,
//! };
//!
//! // Serialize the prover state to a sequence of bytes
//! let bytes = prover_state.to_bytes(false, &gate_serializer, &generator_serializer).unwrap();
//!
//! // Deserialize the bytes into a prover state
//! let recovered_prover_state = AllRecursiveCircuits::<F, C, D>::from_bytes(
//!     &all_circuits_bytes,
//!     false,
//!     &gate_serializer,
//!     &generator_serializer,
//! ).unwrap();
//!
//! assert_eq!(prover_state, recovered_prover_state);
//! ```
//!
//! Note that an entire prover state built with wide ranges may be particularly large (up to ~25 GB), hence serialization methods,
//! while faster than doing another preprocessing, may take some non-negligible time.

#![cfg_attr(docsrs, feature(doc_cfg))]
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

// Public definitions and re-exports

pub type Node = eth_trie_utils::partial_trie::Node<HashedPartialTrie>;

pub use all_stark::AllStark;
pub use config::StarkConfig;
pub use fixed_recursive_verifier::AllRecursiveCircuits;
pub use generation::GenerationInputs;
