use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{Address, BigEndianHash, H160, H256, U256};
use hex_literal::hex;
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
use smt_utils_hermez::code::hash_bytecode_u256;
use smt_utils_hermez::db::{Db, MemoryDb};
use smt_utils_hermez::keys::{key_balance, key_code, key_code_length, key_nonce, key_storage};
use smt_utils_hermez::smt::Smt;
use smt_utils_hermez::utils::hashout2u;

type F = GoldilocksField;
const D: usize = 2;
type C = KeccakGoldilocksConfig;

/// Test a simple selfdestruct.
#[test]
fn test_selfdestruct() -> anyhow::Result<()> {
    init_logger();

    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();

    let beneficiary = hex!("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
    let sender = hex!("5eb96AA102a29fAB267E12A40a5bc6E9aC088759");
    let to = hex!("a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0");

    let sender_account_before = AccountRlp {
        nonce: 5.into(),
        balance: eth_to_wei(100_000.into()),
        ..Default::default()
    };
    let code = vec![
        0x32, // ORIGIN
        0xFF, // SELFDESTRUCT
    ];
    let to_account_before = AccountRlp {
        nonce: 12.into(),
        balance: eth_to_wei(10_000.into()),
        code_hash: hash_bytecode_u256(code.clone()),
        ..Default::default()
    };

    let mut state_smt_before = Smt::<MemoryDb>::default();
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
        transactions_trie: HashedPartialTrie::from(Node::Empty),
        receipts_trie: HashedPartialTrie::from(Node::Empty),
    };

    // Generated using a little py-evm script.
    let txn = hex!("f868050a831e848094a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0880de0b6b3a76400008025a09bab8db7d72e4b42cba8b117883e16872966bae8e4570582de6ed0065e8c36a1a01256d44d982c75e0ab7a19f61ab78afa9e089d51c8686fdfbee085a5ed5d8ff8");

    let block_metadata = BlockMetadata {
        block_beneficiary: Address::from(beneficiary),
        block_timestamp: 0x03e8.into(),
        block_number: 1.into(),
        block_difficulty: 0x020000.into(),
        block_random: H256::from_uint(&0x020000.into()),
        block_gaslimit: 0xff112233u32.into(),
        block_chain_id: 1.into(),
        block_base_fee: 0xa.into(),
        block_gas_used: 26002.into(),
        block_bloom: [0.into(); 8],
    };

    let contract_code = [
        (hash_bytecode_u256(code.clone()), code),
        (hash_bytecode_u256(vec![]), vec![]),
    ]
    .into();

    let expected_state_smt_after = {
        let mut smt = Smt::<MemoryDb>::default();
        let sender_account_after = AccountRlp {
            nonce: 6.into(),
            balance: eth_to_wei(110_000.into()) - 26_002 * 0xa,
            ..Default::default()
        };
        set_account(
            &mut smt,
            H160(sender),
            &sender_account_after,
            &HashMap::new(),
        );
        smt
    };

    let receipt_0 = LegacyReceiptRlp {
        status: true,
        cum_gas_used: 26002.into(),
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
        gas_used_after: 26002.into(),
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

fn eth_to_wei(eth: U256) -> U256 {
    // 1 ether = 10^18 wei.
    eth * U256::from(10).pow(18.into())
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
