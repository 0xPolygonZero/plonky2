use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{Address, BigEndianHash, H160, H256, U256};
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
use smt_utils_hermez::code::hash_contract_bytecode;
use smt_utils_hermez::db::{Db, MemoryDb};
use smt_utils_hermez::keys::{key_balance, key_code, key_code_length, key_nonce, key_storage};
use smt_utils_hermez::smt::Smt;
use smt_utils_hermez::utils::hashout2u;

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

    let beneficiary_nibbles = Nibbles::from_bytes_be(beneficiary_state_key.as_bytes()).unwrap();
    let sender_nibbles = Nibbles::from_bytes_be(sender_state_key.as_bytes()).unwrap();
    let to_nibbles = Nibbles::from_bytes_be(to_hashed.as_bytes()).unwrap();

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
    let code_hash = hashout2u(hash_contract_bytecode(code.to_vec()));

    let beneficiary_account_before = AccountRlp {
        nonce: 1.into(),
        ..AccountRlp::default()
    };
    let sender_account_before = AccountRlp {
        balance: 0x3635c9adc5dea00000u128.into(),
        ..AccountRlp::default()
    };
    let to_account_before = AccountRlp {
        code_hash,
        ..AccountRlp::default()
    };

    let mut state_smt_before = Smt::<MemoryDb>::default();
    set_account(
        &mut state_smt_before,
        H160(beneficiary),
        &beneficiary_account_before,
        &HashMap::new(),
    );
    set_account(
        &mut state_smt_before,
        H160(sender),
        &sender_account_before,
        &HashMap::new(),
    );
    set_account(
        &mut state_smt_before,
        H160(to),
        &to_account_before,
        &HashMap::new(),
    );

    let tries_before = TrieInputs {
        state_smt: state_smt_before.serialize(),
        transactions_trie: Node::Empty.into(),
        receipts_trie: Node::Empty.into(),
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
    contract_code.insert(hashout2u(hash_contract_bytecode(vec![])), vec![]);
    contract_code.insert(code_hash, code.to_vec());

    let expected_state_smt_after = {
        let mut smt = Smt::<MemoryDb>::default();
        let beneficiary_account_after = AccountRlp {
            nonce: 1.into(),
            ..AccountRlp::default()
        };
        let sender_account_after = AccountRlp {
            balance: sender_account_before.balance - U256::from(gas_used) * U256::from(10),
            nonce: 1.into(),
            ..AccountRlp::default()
        };
        let to_account_after = AccountRlp {
            code_hash,
            ..AccountRlp::default()
        };

        set_account(
            &mut smt,
            H160(beneficiary),
            &beneficiary_account_after,
            &HashMap::new(),
        );
        set_account(
            &mut smt,
            H160(sender),
            &sender_account_after,
            &HashMap::new(),
        );
        set_account(
            &mut smt,
            H160(to),
            &to_account_after,
            &HashMap::from([(1.into(), 5.into())]), // Storage map: { 1 => 5 }
        );

        smt
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
        state_root: H256::from_uint(&hashout2u(expected_state_smt_after.root)),
        transactions_root: transactions_trie.hash(),
        receipts_root: receipts_trie.hash(),
    };
    let inputs = GenerationInputs {
        signed_txn: Some(txn.to_vec()),
        withdrawals: vec![],
        tries: tries_before,
        trie_roots_after,
        contract_code,
        checkpoint_state_trie_root: HashedPartialTrie::from(Node::Empty).hash(),
        block_metadata,
        txn_number_before: 0.into(),
        gas_used_before: 0.into(),
        gas_used_after: gas_used.into(),
        block_hashes: BlockHashes {
            prev_hashes: vec![H256::default(); 256],
            cur_hash: H256::default(),
        },
    };

    let mut timing = TimingTree::new("prove", log::Level::Debug);
    let proof = prove::<F, C, D>(&all_stark, &config, inputs, &mut timing, None)?;
    timing.filter(Duration::from_millis(100)).print();

    verify_proof(&all_stark, proof, &config)
}

fn init_logger() {
    let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
}

fn set_account<D: Db>(
    smt: &mut Smt<D>,
    addr: Address,
    account: &AccountRlp,
    storage: &HashMap<U256, U256>,
) {
    smt.set(key_balance(addr), account.balance);
    smt.set(key_nonce(addr), account.nonce);
    smt.set(key_code(addr), account.code_hash);
    smt.set(key_code_length(addr), account.code_length);
    for (&k, &v) in storage {
        smt.set(key_storage(addr, k), v);
    }
}
