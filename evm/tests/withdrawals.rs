use std::collections::HashMap;
use std::time::Duration;

use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{H160, H256, U256};
use keccak_hash::keccak;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::util::timing::TimingTree;
use plonky2_evm::all_stark::AllStark;
use plonky2_evm::config::StarkConfig;
use plonky2_evm::fixed_recursive_verifier::AllRecursiveCircuits;
use plonky2_evm::generation::mpt::AccountRlp;
use plonky2_evm::generation::{GenerationInputs, TrieInputs};
use plonky2_evm::proof::{BlockHashes, BlockMetadata, TrieRoots};
use plonky2_evm::Node;
use rand::random;

type F = GoldilocksField;
const D: usize = 2;
type C = PoseidonGoldilocksConfig;

/// Execute 0 txns and 1 withdrawal.
#[test]
fn test_withdrawals() -> anyhow::Result<()> {
    init_logger();

    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();

    let block_metadata = BlockMetadata::default();

    let state_trie_before = HashedPartialTrie::from(Node::Empty);
    let transactions_trie = HashedPartialTrie::from(Node::Empty);
    let receipts_trie = HashedPartialTrie::from(Node::Empty);
    let storage_tries = vec![];

    let mut contract_code = HashMap::new();
    contract_code.insert(keccak(vec![]), vec![]);

    // Just one withdrawal.
    let withdrawals = vec![(H160(random()), U256(random()))];

    let state_trie_after = {
        let mut trie = HashedPartialTrie::from(Node::Empty);
        let addr_state_key = keccak(withdrawals[0].0);
        let addr_nibbles = Nibbles::from_bytes_be(addr_state_key.as_bytes()).unwrap();
        let account = AccountRlp {
            balance: withdrawals[0].1,
            ..AccountRlp::default()
        };
        trie.insert(addr_nibbles, rlp::encode(&account).to_vec());
        trie
    };

    let trie_roots_after = TrieRoots {
        state_root: state_trie_after.hash(),
        transactions_root: transactions_trie.hash(),
        receipts_root: receipts_trie.hash(),
    };

    let inputs = GenerationInputs {
        signed_txns: vec![],
        withdrawals,
        tries: TrieInputs {
            state_trie: state_trie_before,
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
            prev_hashes: vec![H256::default(); 256],
            cur_hash: H256::default(),
        },
        addresses: vec![],
    };

    let all_circuits = AllRecursiveCircuits::<F, C, D>::new(
        &all_stark,
        &[16..17, 10..11, 15..16, 14..15, 9..10, 12..13, 18..19], // Minimal ranges to prove an empty list
        &config,
    );

    let mut timing = TimingTree::new("prove", log::Level::Info);
    let (root_proof, _public_values) =
        all_circuits.prove_root(&all_stark, &config, inputs, &mut timing)?;
    timing.filter(Duration::from_millis(100)).print();
    all_circuits.verify_root(root_proof.clone())
}

fn init_logger() {
    let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
}
