use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{Address, BigEndianHash, H256};
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
use smt_utils::smt::Smt;

type F = GoldilocksField;
const D: usize = 2;
type C = KeccakGoldilocksConfig;

/// The `add11_yml` test case from https://github.com/ethereum/tests
#[test]
fn add11_yml() -> anyhow::Result<()> {
    init_logger();

    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();

    let beneficiary = hex!("2adc25665018aa1fe0e6bc666dac8fc2697ff9ba");
    let sender = hex!("a94f5374fce5edbc8e2a8697c15331677e6ebf0b");
    let to = hex!("095e7baea6a6c7c4c2dfeb977efac326af552d87");

    let beneficiary_state_key = keccak(beneficiary);
    let sender_state_key = keccak(sender);
    let to_hashed = keccak(to);

    let beneficiary_bits = beneficiary_state_key.into();
    let sender_bits = sender_state_key.into();
    let to_bits = to_hashed.into();

    let code = [0x60, 0x01, 0x60, 0x01, 0x01, 0x60, 0x00, 0x55, 0x00];
    let code_hash = keccak(code);

    let beneficiary_account_before = Account {
        nonce: 1,
        ..Account::default()
    };
    let sender_account_before = Account {
        balance: 0x0de0b6b3a7640000u64.into(),
        ..Account::default()
    };
    let to_account_before = Account {
        balance: 0x0de0b6b3a7640000u64.into(),
        code_hash,
        ..Account::default()
    };

    let mut state_smt_before = Smt::empty();
    state_smt_before
        .insert(beneficiary_bits, beneficiary_account_before.into())
        .unwrap();
    state_smt_before
        .insert(sender_bits, sender_account_before.into())
        .unwrap();
    state_smt_before
        .insert(to_bits, to_account_before.into())
        .unwrap();

    let tries_before = TrieInputs {
        state_trie: state_smt_before.serialize(),
        transactions_trie: Node::Empty.into(),
        receipts_trie: Node::Empty.into(),
        storage_tries: vec![(to_hashed, Node::Empty.into())],
    };

    let txn = hex!("f863800a83061a8094095e7baea6a6c7c4c2dfeb977efac326af552d87830186a0801ba0ffb600e63115a7362e7811894a91d8ba4330e526f22121c994c4692035dfdfd5a06198379fcac8de3dbfac48b165df4bf88e2088f294b61efb9a65fe2281c76e16");

    let block_metadata = BlockMetadata {
        block_beneficiary: Address::from(beneficiary),
        block_timestamp: 0x03e8.into(),
        block_number: 1.into(),
        block_difficulty: 0x020000.into(),
        block_random: H256::from_uint(&0x020000.into()),
        block_gaslimit: 0xff112233u32.into(),
        block_chain_id: 1.into(),
        block_base_fee: 0xa.into(),
        block_gas_used: 0xa868u64.into(),
        block_bloom: [0.into(); 8],
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
            balance: 0xde0b6b3a75be550u64.into(),
            nonce: 1,
            ..Account::default()
        };
        let to_account_after = Account {
            balance: 0xde0b6b3a76586a0u64.into(),
            code_hash,
            // Storage map: { 0 => 2 }
            storage_smt: Smt::new([(keccak([0u8; 32]).into(), 2.into())]).unwrap(),
            ..Account::default()
        };

        let mut expected_state_smt_after = Smt::empty();
        expected_state_smt_after
            .insert(beneficiary_bits, beneficiary_account_after.into())
            .unwrap();
        expected_state_smt_after
            .insert(sender_bits, sender_account_after.into())
            .unwrap();
        expected_state_smt_after
            .insert(to_bits, to_account_after.into())
            .unwrap();
        expected_state_smt_after
    };

    let receipt_0 = LegacyReceiptRlp {
        status: true,
        cum_gas_used: 0xa868u64.into(),
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
        block_metadata,
        genesis_state_trie_root: HashedPartialTrie::from(Node::Empty).hash(),
        txn_number_before: 0.into(),
        gas_used_before: 0.into(),
        gas_used_after: 0xa868u64.into(),
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
