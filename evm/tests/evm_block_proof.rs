#![allow(clippy::upper_case_acronyms)]

use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{Address, U256};
use hex_literal::hex;
use keccak_hash::keccak;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::util::timing::TimingTree;
use plonky2_evm::all_stark::AllStark;
use plonky2_evm::config::StarkConfig;
use plonky2_evm::evm_block_proof::TxnInput;
use plonky2_evm::fixed_recursive_verifier::AllRecursiveCircuits;
use plonky2_evm::generation::mpt::AccountRlp;
use plonky2_evm::generation::{GenerationInputs, TrieInputs};
use plonky2_evm::proof::{BlockMetadata, PublicValues, TrieRoots};
use plonky2_evm::prover::prove;
use plonky2_evm::verifier::verify_proof;
use plonky2_evm::Node;

type F = GoldilocksField;
const D: usize = 2;
type C = PoseidonGoldilocksConfig;

/// Test a block proof for a block consisting of two simple transfers.
#[test]
fn test_block_proof() -> anyhow::Result<()> {
    init_logger();

    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();

    let beneficiary = hex!("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
    let sender = hex!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
    let to0 = hex!("4675C7e5BaAFBFFbca748158bEcBA61ef3b0a263");
    let to1 = hex!("6ACd5490B675cb9525ed44bA6BEB7E7Ae526ED16");

    let sender_state_key = keccak(sender);
    let to0_state_key = keccak(to0);
    let to1_state_key = keccak(to1);

    let sender_nibbles = Nibbles::from_bytes_be(sender_state_key.as_bytes()).unwrap();
    let to0_nibbles = Nibbles::from_bytes_be(to0_state_key.as_bytes()).unwrap();
    let to1_nibbles = Nibbles::from_bytes_be(to1_state_key.as_bytes()).unwrap();

    let sender_account_before = AccountRlp {
        nonce: 0.into(),
        balance: eth_to_wei(10_000.into()),
        storage_root: HashedPartialTrie::from(Node::Empty).hash(),
        code_hash: keccak([]),
    };
    let to0_account_before = AccountRlp::default();
    let to1_account_before = AccountRlp::default();

    let state_trie_before = Node::Leaf {
        nibbles: sender_nibbles,
        value: rlp::encode(&sender_account_before).to_vec(),
    }
    .into();
    let tries_before_txn0 = TrieInputs {
        state_trie: state_trie_before,
        transactions_trie: HashedPartialTrie::from(Node::Empty),
        receipts_trie: HashedPartialTrie::from(Node::Empty),
        storage_tries: vec![],
    };

    // Generated with Python script.
    let txn0 = hex!("f86c80850ba43b7400825208944675c7e5baafbffbca748158becba61ef3b0a263880de0b6b3a7640000801ba003cc53dd2a5fa38720a3c39c766c51265f8666a2605afe7a1abafec23d65198ba04c7978af9ab1fc6ff2efa611a508881c66d829867ad767764938f9387b5a3da9");
    let txn1 = hex!("f86c01850ba43b7400825208946acd5490b675cb9525ed44ba6beb7e7ae526ed16880de0b6b3a7640000801ba04bc87c6077d52bb3a21e45c3892e71ee93d2dfff4f31fb471fc944faa359f3faa07565b327297a2f2f84ef42ba8bc7b5fe4200d2e9d451283d3a430df33e40ccef");

    let value = eth_to_wei(1.into());

    let block_metadata = BlockMetadata {
        block_beneficiary: Address::from(beneficiary),
        block_timestamp: 1690880814.into(),
        block_number: 1.into(),
        block_difficulty: 0x020000.into(),
        block_gaslimit: 30000000.into(),
        block_chain_id: 31337.into(),
        block_base_fee: 50_000_000_000usize.into(),
    };

    let mut contract_code = HashMap::new();
    contract_code.insert(keccak(vec![]), vec![]);

    let txn0_inps = TxnInput {
        signed_txn: txn0.to_vec(),
        tries: tries_before_txn0.clone(),
        contract_code: contract_code.clone(),
    };

    let all_circuits = AllRecursiveCircuits::<F, C, D>::new(
        &all_stark,
        &[16..17, 14..18, 14..15, 9..10, 12..13, 18..20],
        &config,
    );

    let expected_state_trie_after_txn0: HashedPartialTrie = {
        let gas_used = 21_000;

        let sender_account_after = AccountRlp {
            balance: sender_account_before.balance - value - gas_used * 50_000_000_000usize,
            nonce: sender_account_before.nonce + 1,
            ..sender_account_before
        };
        let to0_account_after = AccountRlp {
            balance: value,
            ..to0_account_before
        };

        assert_ne!(sender_nibbles.get_nibble(0), to0_nibbles.get_nibble(0));
        let mut children = core::array::from_fn(|_| Node::Empty.into());
        children[sender_nibbles.get_nibble(0) as usize] = Node::Leaf {
            nibbles: sender_nibbles.truncate_n_nibbles_front(1),
            value: rlp::encode(&sender_account_after).to_vec(),
        }
        .into();
        children[to0_nibbles.get_nibble(0) as usize] = Node::Leaf {
            nibbles: to0_nibbles.truncate_n_nibbles_front(1),
            value: rlp::encode(&to0_account_after).to_vec(),
        }
        .into();
        Node::Branch {
            children: children.clone(),
            value: vec![],
        }
        .into()
    };

    let mut timing = TimingTree::new("prove", log::Level::Debug);
    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();

    // Prove the block consisting of only txn0.
    let proof = all_circuits.prove_evm_block(
        vec![txn0_inps.clone()],
        block_metadata.clone(),
        PublicValues {
            trie_roots_before: tries_before_txn0.clone().into(),
            trie_roots_after: TrieRoots {
                state_root: expected_state_trie_after_txn0.hash(),
                transactions_root: HashedPartialTrie::from(Node::Empty).hash(), // TODO: fix this when we have transactions trie
                receipts_root: HashedPartialTrie::from(Node::Empty).hash(), // TODO: fix this when we have receipts trie
            },
            block_metadata: block_metadata.clone(),
        },
        &all_stark,
        &config,
        &mut timing,
    )?;
    timing.filter(Duration::from_millis(100)).print();
    all_circuits.verify_evm_block(&proof)?;

    let tries_before_txn1 = TrieInputs {
        state_trie: expected_state_trie_after_txn0,
        transactions_trie: HashedPartialTrie::from(Node::Empty),
        receipts_trie: HashedPartialTrie::from(Node::Empty),
        storage_tries: vec![],
    };
    let txn1_inps = TxnInput {
        signed_txn: txn1.to_vec(),
        tries: tries_before_txn1.clone(),
        contract_code,
    };

    let expected_state_trie_after_txn1: HashedPartialTrie = {
        let gas_used = 21_000;

        let sender_account_after = AccountRlp {
            balance: sender_account_before.balance - value * 2 - 2 * gas_used * 50_000_000_000usize,
            nonce: 2.into(),
            ..sender_account_before
        };
        let to0_account_after = AccountRlp {
            balance: value,
            ..to0_account_before
        };

        let to1_account_after = AccountRlp {
            balance: value,
            ..to1_account_before
        };

        assert_ne!(sender_nibbles.get_nibble(0), to0_nibbles.get_nibble(0));
        assert_ne!(sender_nibbles.get_nibble(0), to1_nibbles.get_nibble(0));
        assert_ne!(to0_nibbles.get_nibble(0), to1_nibbles.get_nibble(0));
        let mut children = core::array::from_fn(|_| Node::Empty.into());
        children[sender_nibbles.get_nibble(0) as usize] = Node::Leaf {
            nibbles: sender_nibbles.truncate_n_nibbles_front(1),
            value: rlp::encode(&sender_account_after).to_vec(),
        }
        .into();
        children[to0_nibbles.get_nibble(0) as usize] = Node::Leaf {
            nibbles: to0_nibbles.truncate_n_nibbles_front(1),
            value: rlp::encode(&to0_account_after).to_vec(),
        }
        .into();
        children[to1_nibbles.get_nibble(0) as usize] = Node::Leaf {
            nibbles: to1_nibbles.truncate_n_nibbles_front(1),
            value: rlp::encode(&to1_account_after).to_vec(),
        }
        .into();
        Node::Branch {
            children,
            value: vec![],
        }
        .into()
    };

    // Prove the block consisting of txn0 and txn1.
    let proof = all_circuits.prove_evm_block(
        vec![txn0_inps, txn1_inps],
        block_metadata.clone(),
        PublicValues {
            trie_roots_before: tries_before_txn0.into(),
            trie_roots_after: TrieRoots {
                state_root: expected_state_trie_after_txn1.hash(),
                transactions_root: HashedPartialTrie::from(Node::Empty).hash(), // TODO: fix this when we have transactions trie
                receipts_root: HashedPartialTrie::from(Node::Empty).hash(), // TODO: fix this when we have receipts trie
            },
            block_metadata,
        },
        &all_stark,
        &config,
        &mut timing,
    )?;
    timing.filter(Duration::from_millis(100)).print();
    all_circuits.verify_evm_block(&proof)
}

fn eth_to_wei(eth: U256) -> U256 {
    // 1 ether = 10^18 wei.
    eth * U256::from(10).pow(18.into())
}

fn init_logger() {
    let _ = try_init_from_env(Env::default().filter_or(DEFAULT_FILTER_ENV, "info"));
}
