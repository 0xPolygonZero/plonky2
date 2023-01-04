use std::collections::HashMap;
use std::time::Duration;

use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
use eth_trie_utils::partial_trie::PartialTrie;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::util::timing::TimingTree;
use plonky2_evm::all_stark::AllStark;
use plonky2_evm::config::StarkConfig;
use plonky2_evm::fixed_recursive_verifier::AllRecursiveCircuits;
use plonky2_evm::generation::{GenerationInputs, TrieInputs};
use plonky2_evm::proof::BlockMetadata;
use plonky2_evm::prover::prove;
use plonky2_evm::verifier::verify_proof;

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

    let block_metadata = BlockMetadata::default();

    let state_trie = PartialTrie::Empty;
    let transactions_trie = PartialTrie::Empty;
    let receipts_trie = PartialTrie::Empty;
    let storage_tries = vec![];

    let state_trie_root = state_trie.calc_hash();
    let txns_trie_root = transactions_trie.calc_hash();
    let receipts_trie_root = receipts_trie.calc_hash();

    let inputs = GenerationInputs {
        signed_txns: vec![],
        tries: TrieInputs {
            state_trie,
            transactions_trie,
            receipts_trie,
            storage_tries,
        },
        contract_code: HashMap::new(),
        block_metadata,
    };

    let mut timing = TimingTree::new("prove", log::Level::Debug);
    // TODO: This is redundant; prove_root below calls this prove method internally.
    // Just keeping it for now because the root proof returned by prove_root doesn't contain public
    // values yet, and we want those for the assertions below.
    let proof = prove::<F, C, D>(&all_stark, &config, inputs.clone(), &mut timing)?;
    timing.filter(Duration::from_millis(100)).print();

    assert_eq!(
        proof.public_values.trie_roots_before.state_root,
        state_trie_root
    );
    assert_eq!(
        proof.public_values.trie_roots_after.state_root,
        state_trie_root
    );
    assert_eq!(
        proof.public_values.trie_roots_before.transactions_root,
        txns_trie_root
    );
    assert_eq!(
        proof.public_values.trie_roots_after.transactions_root,
        txns_trie_root
    );
    assert_eq!(
        proof.public_values.trie_roots_before.receipts_root,
        receipts_trie_root
    );
    assert_eq!(
        proof.public_values.trie_roots_after.receipts_root,
        receipts_trie_root
    );

    verify_proof(&all_stark, proof, &config)?;

    let all_circuits = AllRecursiveCircuits::<F, C, D>::new(&all_stark, 9..19, &config);
    let root_proof = all_circuits.prove_root(&all_stark, &config, inputs, &mut timing)?;
    all_circuits.verify_root(root_proof.clone())?;

    let agg_proof = all_circuits.prove_aggregation(false, &root_proof, false, &root_proof)?;
    all_circuits.verify_aggregation(&agg_proof)
}

fn init_logger() {
    let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
}
