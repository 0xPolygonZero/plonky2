use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::Address;
use hex_literal::hex;
use keccak_hash::keccak;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::util::timing::TimingTree;
use plonky2_evm::all_stark::AllStark;
use plonky2_evm::config::StarkConfig;
use plonky2_evm::generation::mpt::AccountRlp;
use plonky2_evm::generation::{GenerationInputs, TrieInputs};
use plonky2_evm::proof::BlockMetadata;
use plonky2_evm::prover::prove;
use plonky2_evm::verifier::verify_proof;
use plonky2_evm::Node;

type F = GoldilocksField;
const D: usize = 2;
type C = PoseidonGoldilocksConfig;

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
    let code_hash = keccak(code);

    let beneficiary_account_before = AccountRlp::default();
    let sender_account_before = AccountRlp {
        balance: 0x3635c9adc5dea00000u128.into(),
        ..AccountRlp::default()
    };
    let to_account_before = AccountRlp {
        code_hash,
        ..AccountRlp::default()
    };

    let mut state_trie_before = HashedPartialTrie::from(Node::Empty);
    state_trie_before.insert(
        beneficiary_nibbles,
        rlp::encode(&beneficiary_account_before).to_vec(),
    );
    state_trie_before.insert(sender_nibbles, rlp::encode(&sender_account_before).to_vec());
    state_trie_before.insert(to_nibbles, rlp::encode(&to_account_before).to_vec());

    let tries_before = TrieInputs {
        state_trie: state_trie_before,
        transactions_trie: Node::Empty.into(),
        receipts_trie: Node::Empty.into(),
        storage_tries: vec![(to_hashed, Node::Empty.into())],
    };

    let txn = hex!("f861800a8405f5e10094100000000000000000000000000000000000000080801ba07e09e26678ed4fac08a249ebe8ed680bf9051a5e14ad223e4b2b9d26e0208f37a05f6e3f188e3e6eab7d7d3b6568f5eac7d687b08d307d3154ccd8c87b4630509b");

    let block_metadata = BlockMetadata {
        block_beneficiary: Address::from(beneficiary),
        block_base_fee: 0xa.into(),
        ..BlockMetadata::default()
    };

    let mut contract_code = HashMap::new();
    contract_code.insert(keccak(vec![]), vec![]);
    contract_code.insert(code_hash, code.to_vec());

    let inputs = GenerationInputs {
        signed_txns: vec![txn.to_vec()],
        tries: tries_before,
        contract_code,
        block_metadata,
        addresses: vec![],
    };

    let mut timing = TimingTree::new("prove", log::Level::Debug);
    let proof = prove::<F, C, D>(&all_stark, &config, inputs, &mut timing)?;
    timing.filter(Duration::from_millis(100)).print();

    let beneficiary_account_after = AccountRlp::default();
    let sender_account_after = AccountRlp {
        balance: 999999999999999568680u128.into(),
        nonce: 1.into(),
        ..AccountRlp::default()
    };
    let to_account_after = AccountRlp {
        code_hash,
        // Storage map: { 1 => 5 }
        storage_root: HashedPartialTrie::from(Node::Leaf {
            // TODO: Could do keccak(pad32(1))
            nibbles: Nibbles::from_str(
                "0xb10e2d527612073b26eecdfd717e6a320cf44b4afac2b0732d9fcbe2b7fa0cf6",
            )
            .unwrap(),
            value: vec![5],
        })
        .hash(),
        ..AccountRlp::default()
    };

    let mut expected_state_trie_after = HashedPartialTrie::from(Node::Empty);
    expected_state_trie_after.insert(
        beneficiary_nibbles,
        rlp::encode(&beneficiary_account_after).to_vec(),
    );
    expected_state_trie_after.insert(sender_nibbles, rlp::encode(&sender_account_after).to_vec());
    expected_state_trie_after.insert(to_nibbles, rlp::encode(&to_account_after).to_vec());

    assert_eq!(
        proof.public_values.trie_roots_after.state_root,
        expected_state_trie_after.hash()
    );

    verify_proof(&all_stark, proof, &config)
}

fn init_logger() {
    let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
}
