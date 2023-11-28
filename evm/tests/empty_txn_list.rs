use std::collections::HashMap;
use std::marker::PhantomData;
use std::time::Duration;

use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{BigEndianHash, H256};
use keccak_hash::keccak;
use log::info;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::util::serialization::{DefaultGateSerializer, DefaultGeneratorSerializer};
use plonky2::util::timing::TimingTree;
use plonky2_evm::all_stark::AllStark;
use plonky2_evm::config::StarkConfig;
use plonky2_evm::fixed_recursive_verifier::AllRecursiveCircuits;
use plonky2_evm::generation::{GenerationInputs, TrieInputs};
use plonky2_evm::proof::{BlockHashes, BlockMetadata, TrieRoots};
use plonky2_evm::Node;

type F = GoldilocksField;
const D: usize = 2;
type C = PoseidonGoldilocksConfig;

/// Execute the empty list of transactions, i.e. a no-op.
#[test]
#[ignore] // Too slow to run on CI.
fn test_empty_txn_list() -> anyhow::Result<()> {
    init_logger();

    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();

    let block_metadata = BlockMetadata {
        block_number: 1.into(),
        ..Default::default()
    };

    let state_trie = HashedPartialTrie::from(Node::Empty);
    let transactions_trie = HashedPartialTrie::from(Node::Empty);
    let receipts_trie = HashedPartialTrie::from(Node::Empty);
    let storage_tries = vec![];

    let mut contract_code = HashMap::new();
    contract_code.insert(keccak(vec![]), vec![]);

    // No transactions, so no trie roots change.
    let trie_roots_after = TrieRoots {
        state_root: state_trie.hash(),
        transactions_root: transactions_trie.hash(),
        receipts_root: receipts_trie.hash(),
    };
    let mut initial_block_hashes = vec![H256::default(); 256];
    initial_block_hashes[255] = H256::from_uint(&0x200.into());
    let inputs = GenerationInputs {
        signed_txn: None,
        withdrawals: vec![],
        tries: TrieInputs {
            state_trie,
            transactions_trie,
            receipts_trie,
            storage_tries,
        },
        trie_roots_after,
        contract_code,
        genesis_state_trie_root: HashedPartialTrie::from(Node::Empty).hash(),
        block_metadata,
        txn_number_before: 0.into(),
        gas_used_before: 0.into(),
        gas_used_after: 0.into(),
        block_bloom_before: [0.into(); 8],
        block_bloom_after: [0.into(); 8],
        block_hashes: BlockHashes {
            prev_hashes: initial_block_hashes,
            cur_hash: H256::default(),
        },
        addresses: vec![],
    };

    // Initialize the preprocessed circuits for the zkEVM.
    // The provided ranges are the minimal ones to prove an empty list, except the one of the CPU
    // that is wrong for testing purposes, see below.
    let mut all_circuits = AllRecursiveCircuits::<F, C, D>::new(
        &all_stark,
        &[16..17, 10..11, 12..13, 14..15, 9..11, 12..13, 18..19], // Minimal ranges to prove an empty list
        &config,
    );

    {
        let gate_serializer = DefaultGateSerializer;
        let generator_serializer = DefaultGeneratorSerializer {
            _phantom: PhantomData::<C>,
        };

        let timing = TimingTree::new("serialize AllRecursiveCircuits", log::Level::Info);
        let all_circuits_bytes = all_circuits
            .to_bytes(&gate_serializer, &generator_serializer)
            .map_err(|_| anyhow::Error::msg("AllRecursiveCircuits serialization failed."))?;
        timing.filter(Duration::from_millis(100)).print();
        info!(
            "AllRecursiveCircuits length: {} bytes",
            all_circuits_bytes.len()
        );

        let timing = TimingTree::new("deserialize AllRecursiveCircuits", log::Level::Info);
        let all_circuits_from_bytes = AllRecursiveCircuits::<F, C, D>::from_bytes(
            &all_circuits_bytes,
            &gate_serializer,
            &generator_serializer,
        )
        .map_err(|_| anyhow::Error::msg("AllRecursiveCircuits deserialization failed."))?;
        timing.filter(Duration::from_millis(100)).print();

        assert_eq!(all_circuits, all_circuits_from_bytes);
    }

    let mut timing = TimingTree::new("prove", log::Level::Info);
    // We're missing some preprocessed circuits.
    assert!(all_circuits
        .prove_root(&all_stark, &config, inputs.clone(), &mut timing)
        .is_err());

    // Expand the preprocessed circuits.
    // We pass an empty range if we don't want to add different table sizes.
    all_circuits.expand(
        &all_stark,
        &[0..0, 0..0, 15..16, 0..0, 0..0, 0..0, 0..0],
        &StarkConfig::standard_fast_config(),
    );

    let mut timing = TimingTree::new("prove", log::Level::Info);
    let (root_proof, public_values) =
        all_circuits.prove_root(&all_stark, &config, inputs, &mut timing)?;
    timing.filter(Duration::from_millis(100)).print();
    all_circuits.verify_root(root_proof.clone())?;

    // We can duplicate the proofs here because the state hasn't mutated.
    let (agg_proof, public_values) = all_circuits.prove_aggregation(
        false,
        &root_proof,
        public_values.clone(),
        false,
        &root_proof,
        public_values,
    )?;
    all_circuits.verify_aggregation(&agg_proof)?;

    let (block_proof, _) = all_circuits.prove_block(None, &agg_proof, public_values)?;
    all_circuits.verify_block(&block_proof)?;

    // Get the verifier associated to these preprocessed circuits, and have it verify the block_proof.
    let verifier = all_circuits.final_verifier_data();
    verifier.verify(block_proof)
}

fn init_logger() {
    let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
}
