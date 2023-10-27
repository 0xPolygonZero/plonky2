use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{Address, BigEndianHash, H256, U256};
use hex_literal::hex;
use keccak_hash::keccak;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::KeccakGoldilocksConfig;
use plonky2::util::timing::TimingTree;
use plonky2_evm::all_stark::AllStark;
use plonky2_evm::config::StarkConfig;
use plonky2_evm::generation::mpt::{AccountRlp, LegacyReceiptRlp};
use plonky2_evm::generation::{GenerationInputs, TrieInputs};
use plonky2_evm::proof::{BlockHashes, BlockMetadata, TrieRoots};
use plonky2_evm::prover::prove;
use plonky2_evm::verifier::verify_proof;
use plonky2_evm::Node;
use smt_utils::account::Account;
use smt_utils::smt::Smt;

type F = GoldilocksField;
const D: usize = 2;
type C = KeccakGoldilocksConfig;

/// The `selfBalanceGasCost` test case from https://github.com/ethereum/tests
#[test]
#[ignore] // Too slow to run on CI.
fn self_balance_gas_cost() -> anyhow::Result<()> {
    init_logger();

    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();

    let beneficiary = hex!("2adc25665018aa1fe0e6bc666dac8fc2697ff9ba");
    let sender = hex!("a94f5374fce5edbc8e2a8697c15331677e6ebf0b");
    let to = hex!("1000000000000000000000000000000000000000");

    let beneficiary_state_key = keccak(beneficiary);
    let sender_state_key = keccak(sender);
    let to_hashed = keccak(to);

    let beneficiary_bits = beneficiary_state_key.into_uint().into();
    let sender_bits = sender_state_key.into_uint().into();
    let to_bits = to_hashed.into_uint().into();

    let code = [
        0x5a, 0x47, 0x5a, 0x90, 0x50, 0x90, 0x03, 0x60, 0x02, 0x90, 0x03, 0x60, 0x01, 0x55, 0x00,
    ];
    let code_gas = 2 // GAS
    + 5 // SELFBALANCE
    + 2 // GAS
    + 3 // SWAP1
    + 2 // POP
    + 3 // SWAP1
    + 3 // SUB
    + 3 // PUSH1
    + 3 // SWAP1
    + 3 // SUB
    + 3 // PUSH1
    + 22100; // SSTORE
    let code_hash = keccak(code);

    let beneficiary_account_before = Account {
        nonce: 1,
        ..Account::default()
    };
    let sender_account_before = Account {
        balance: 0x3635c9adc5dea00000u128.into(),
        ..Account::default()
    };
    let to_account_before = Account {
        code_hash,
        ..Account::default()
    };

    let state_smt_before = Smt::new([
        (beneficiary_bits, beneficiary_account_before.clone().into()),
        (sender_bits, sender_account_before.clone().into()),
        (to_bits, to_account_before.clone().into()),
    ])
    .unwrap();

    let tries_before = TrieInputs {
        state_trie: state_smt_before.serialize(),
        transactions_trie: Node::Empty.into(),
        receipts_trie: Node::Empty.into(),
        storage_tries: vec![(to_hashed, Node::Empty.into())],
    };

    let txn = hex!("f861800a8405f5e10094100000000000000000000000000000000000000080801ba07e09e26678ed4fac08a249ebe8ed680bf9051a5e14ad223e4b2b9d26e0208f37a05f6e3f188e3e6eab7d7d3b6568f5eac7d687b08d307d3154ccd8c87b4630509b");

    let gas_used = 21_000 + code_gas;

    let block_metadata = BlockMetadata {
        block_beneficiary: Address::from(beneficiary),
        block_difficulty: 0x20000.into(),
        block_number: 1.into(),
        block_chain_id: 1.into(),
        block_timestamp: 0x03e8.into(),
        block_gaslimit: 0xff112233u32.into(),
        block_gas_used: gas_used.into(),
        block_bloom: [0.into(); 8],
        block_base_fee: 0xa.into(),
        block_random: Default::default(),
    };

    let mut contract_code = HashMap::new();
    contract_code.insert(keccak(vec![]), vec![]);
    contract_code.insert(code_hash, code.to_vec());

    let expected_state_trie_after = {
        let beneficiary_account_after = Account {
            nonce: 1,
            ..Account::default()
        };
        let sender_account_after = Account {
            balance: sender_account_before.balance - U256::from(gas_used) * U256::from(10),
            nonce: 1,
            ..Account::default()
        };
        let to_account_after = Account {
            code_hash,
            // Storage map: { 1 => 5 }
            storage_smt: Smt::new([(
                U256::from_str(
                    "0xb10e2d527612073b26eecdfd717e6a320cf44b4afac2b0732d9fcbe2b7fa0cf6", // keccak(pad(1))
                )
                .unwrap()
                .into(),
                U256::from(5).into(),
            )])
            .unwrap(),
            ..Account::default()
        };

        Smt::new([
            (beneficiary_bits, beneficiary_account_after.into()),
            (sender_bits, sender_account_after.into()),
            (to_bits, to_account_after.into()),
        ])
        .unwrap()
    };

    let receipt_0 = LegacyReceiptRlp {
        status: true,
        cum_gas_used: gas_used.into(),
        bloom: vec![0; 256].into(),
        logs: vec![],
    };
    let mut receipts_trie = HashedPartialTrie::from(Node::Empty);
    receipts_trie.insert(
        Nibbles::from_str("0x80").unwrap(),
        rlp::encode(&receipt_0).to_vec(),
    );
    let transactions_trie: HashedPartialTrie = Node::Leaf {
        nibbles: Nibbles::from_str("0x80").unwrap(),
        value: txn.to_vec(),
    }
    .into();

    let trie_roots_after = TrieRoots {
        state_root: expected_state_trie_after.root,
        transactions_root: transactions_trie.hash(),
        receipts_root: receipts_trie.hash(),
    };
    let inputs = GenerationInputs {
        signed_txns: vec![txn.to_vec()],
        tries: tries_before,
        trie_roots_after,
        contract_code,
        genesis_state_trie_root: HashedPartialTrie::from(Node::Empty).hash(),
        block_metadata,
        txn_number_before: 0.into(),
        gas_used_before: 0.into(),
        gas_used_after: gas_used.into(),
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
