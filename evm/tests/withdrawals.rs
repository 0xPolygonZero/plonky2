use std::collections::HashMap;
use std::time::Duration;

use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{H160, H256, U256};
use keccak_hash::keccak;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::util::timing::TimingTree;
use plonky2_evm::all_stark::AllStark;
use plonky2_evm::config::StarkConfig;
use plonky2_evm::generation::{GenerationInputs, TrieInputs};
use plonky2_evm::proof::{BlockHashes, BlockMetadata, TrieRoots};
use plonky2_evm::prover::prove;
use plonky2_evm::verifier::verify_proof;
use plonky2_evm::Node;
use rand::random;
use smt_utils::account::Account;
use smt_utils::bits::Bits;
use smt_utils::smt::Smt;

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

    let state_smt_before = Smt::empty();
    let transactions_trie = HashedPartialTrie::from(Node::Empty);
    let receipts_trie = HashedPartialTrie::from(Node::Empty);

    let mut contract_code = HashMap::new();
    contract_code.insert(keccak(vec![]), vec![]);

    // Just one withdrawal.
    let withdrawals = vec![(H160(random()), U256(random()))];

    let state_smt_after = {
        let mut smt = Smt::empty();
        let addr_state_key = keccak(withdrawals[0].0);
        let addr_bits = Bits::from(addr_state_key);
        let account = Account {
            balance: withdrawals[0].1,
            ..Account::default()
        };
        smt.insert(addr_bits, account.into()).unwrap();
        smt
    };

    let trie_roots_after = TrieRoots {
        state_root: state_smt_after.root,
        transactions_root: transactions_trie.hash(),
        receipts_root: receipts_trie.hash(),
    };

    let inputs = GenerationInputs {
        signed_txn: None,
        withdrawals,
        tries: TrieInputs {
            state_smt: state_smt_before.serialize(),
            transactions_trie,
            receipts_trie,
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

    let mut timing = TimingTree::new("prove", log::Level::Debug);
    let proof = prove::<F, C, D>(&all_stark, &config, inputs, &mut timing)?;
    timing.filter(Duration::from_millis(100)).print();

    verify_proof(&all_stark, proof, &config)
}

fn init_logger() {
    let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
}
