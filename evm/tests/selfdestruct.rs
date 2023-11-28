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
use plonky2_evm::generation::mpt::LegacyReceiptRlp;
use plonky2_evm::generation::{GenerationInputs, TrieInputs};
use plonky2_evm::proof::{BlockHashes, BlockMetadata, TrieRoots};
use plonky2_evm::prover::prove;
use plonky2_evm::verifier::verify_proof;
use plonky2_evm::Node;
use smt_utils::account::Account;
use smt_utils::bits::Bits;
use smt_utils::smt::Smt;

type F = GoldilocksField;
const D: usize = 2;
type C = KeccakGoldilocksConfig;

/// Test a simple selfdestruct.
#[ignore]
#[test]
fn test_selfdestruct() -> anyhow::Result<()> {
    init_logger();

    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();

    let beneficiary = hex!("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
    let sender = hex!("5eb96AA102a29fAB267E12A40a5bc6E9aC088759");
    let to = hex!("a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0");

    let sender_state_key = keccak(sender);
    let to_state_key = keccak(to);

    let sender_bits = Bits::from(sender_state_key);
    let to_bits = Bits::from(to_state_key);

    let sender_account_before = Account {
        nonce: 5,
        balance: eth_to_wei(100_000.into()),
        code_hash: keccak([]),
        storage_smt: Smt::empty(),
    };
    let code = vec![
        0x32, // ORIGIN
        0xFF, // SELFDESTRUCT
    ];
    let to_account_before = Account {
        nonce: 12,
        balance: eth_to_wei(10_000.into()),
        code_hash: keccak(&code),
        storage_smt: Smt::empty(),
    };

    let state_trie_before = Smt::new([
        (sender_bits, sender_account_before.into()),
        (to_bits, to_account_before.into()),
    ])
    .unwrap();

    let tries_before = TrieInputs {
        state_smt: state_trie_before.serialize(),
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

    let contract_code = [(keccak(&code), code), (keccak([]), vec![])].into();

    let expected_state_trie_after = {
        let sender_account_after = Account {
            nonce: 6,
            balance: eth_to_wei(110_000.into()) - 26_002 * 0xa,
            code_hash: keccak([]),
            storage_smt: Smt::empty(),
        };
        Smt::new([(sender_bits, sender_account_after.into())]).unwrap()
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
        state_root: expected_state_trie_after.root,
        transactions_root: transactions_trie.hash(),
        receipts_root: receipts_trie.hash(),
    };
    let inputs = GenerationInputs {
        signed_txn: Some(txn.to_vec()),
        withdrawals: vec![],
        tries: tries_before,
        trie_roots_after,
        contract_code,
        genesis_state_trie_root: HashedPartialTrie::from(Node::Empty).hash(),
        block_metadata,
        txn_number_before: 0.into(),
        gas_used_before: 0.into(),
        gas_used_after: 26002.into(),
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

fn eth_to_wei(eth: U256) -> U256 {
    // 1 ether = 10^18 wei.
    eth * U256::from(10).pow(18.into())
}

fn init_logger() {
    let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
}
