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
use plonky2_evm::proof::{BlockHashes, BlockMetadata, PublicValues, TrieRoots};
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
        checkpoint_state_trie_root: HashedPartialTrie::from(Node::Empty).hash(),
        block_metadata,
        txn_number_before: 0.into(),
        gas_used_before: 0.into(),
        gas_used_after: 0.into(),
        block_hashes: BlockHashes {
            prev_hashes: initial_block_hashes,
            cur_hash: H256::default(),
        },
    };

    // Initialize the preprocessed circuits for the zkEVM.
    let all_circuits = AllRecursiveCircuits::<F, C, D>::new(
        &all_stark,
        &[16..17, 9..11, 12..13, 14..15, 9..11, 12..13, 17..18], // Minimal ranges to prove an empty list
        &config,
    );

    {
        let gate_serializer = DefaultGateSerializer;
        let generator_serializer = DefaultGeneratorSerializer::<C, D> {
            _phantom: PhantomData::<C>,
        };

        let timing = TimingTree::new("serialize AllRecursiveCircuits", log::Level::Info);
        let all_circuits_bytes = all_circuits
            .to_bytes(false, &gate_serializer, &generator_serializer)
            .map_err(|_| anyhow::Error::msg("AllRecursiveCircuits serialization failed."))?;
        timing.filter(Duration::from_millis(100)).print();
        info!(
            "AllRecursiveCircuits length: {} bytes",
            all_circuits_bytes.len()
        );

        let timing = TimingTree::new("deserialize AllRecursiveCircuits", log::Level::Info);
        let all_circuits_from_bytes = AllRecursiveCircuits::<F, C, D>::from_bytes(
            &all_circuits_bytes,
            false,
            &gate_serializer,
            &generator_serializer,
        )
        .map_err(|_| anyhow::Error::msg("AllRecursiveCircuits deserialization failed."))?;
        timing.filter(Duration::from_millis(100)).print();

        assert_eq!(all_circuits, all_circuits_from_bytes);
    }

    let mut timing = TimingTree::new("prove", log::Level::Info);
    let (root_proof, public_values) =
        all_circuits.prove_root(&all_stark, &config, inputs, &mut timing, None)?;
    timing.filter(Duration::from_millis(100)).print();
    all_circuits.verify_root(root_proof.clone())?;

    // Test retrieved public values from the proof public inputs.
    let retrieved_public_values = PublicValues::from_public_inputs(&root_proof.public_inputs);
    assert_eq!(retrieved_public_values, public_values);

    // We can duplicate the proofs here because the state hasn't mutated.
    let (agg_proof, agg_public_values) = all_circuits.prove_aggregation(
        false,
        &root_proof,
        public_values.clone(),
        false,
        &root_proof,
        public_values,
    )?;
    all_circuits.verify_aggregation(&agg_proof)?;

    // Test retrieved public values from the proof public inputs.
    let retrieved_public_values = PublicValues::from_public_inputs(&agg_proof.public_inputs);
    assert_eq!(retrieved_public_values, agg_public_values);

    let (block_proof, block_public_values) =
        all_circuits.prove_block(None, &agg_proof, agg_public_values)?;
    all_circuits.verify_block(&block_proof)?;

    // Test retrieved public values from the proof public inputs.
    let retrieved_public_values = PublicValues::from_public_inputs(&block_proof.public_inputs);
    assert_eq!(retrieved_public_values, block_public_values);

    // Get the verifier associated to these preprocessed circuits, and have it verify the block_proof.
    let verifier = all_circuits.final_verifier_data();
    verifier.verify(block_proof)
}

fn init_logger() {
    let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
}
