#![allow(clippy::upper_case_acronyms)]

use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use env_logger::{try_init_from_env, Env, DEFAULT_FILTER_ENV};
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{Address, H256, U256};
use hex_literal::hex;
use keccak_hash::keccak;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::KeccakGoldilocksConfig;
use plonky2::util::timing::TimingTree;
use plonky2_evm::all_stark::AllStark;
use plonky2_evm::config::StarkConfig;
use plonky2_evm::cpu::kernel::opcodes::{get_opcode, get_push_opcode};
use plonky2_evm::generation::mpt::{AccountRlp, LegacyReceiptRlp};
use plonky2_evm::generation::{GenerationInputs, TrieInputs};
use plonky2_evm::proof::{BlockHashes, BlockMetadata, TrieRoots};
use plonky2_evm::prover::prove;
use plonky2_evm::verifier::verify_proof;
use plonky2_evm::Node;

type F = GoldilocksField;
const D: usize = 2;
type C = KeccakGoldilocksConfig;

/// Test the validity of four transactions, where only the first one is valid and the other three abort.  
#[test]
fn test_four_transactions() -> anyhow::Result<()> {
    init_logger();

    let all_stark = AllStark::<F, D>::default();
    let config = StarkConfig::standard_fast_config();

    let beneficiary = hex!("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
    let sender = hex!("2c7536e3605d9c16a7a3d7b1898e529396a65c23");
    let to = hex!("a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0");

    let beneficiary_state_key = keccak(beneficiary);
    let sender_state_key = keccak(sender);
    let to_state_key = keccak(to);

    let beneficiary_nibbles = Nibbles::from_bytes_be(beneficiary_state_key.as_bytes()).unwrap();
    let sender_nibbles = Nibbles::from_bytes_be(sender_state_key.as_bytes()).unwrap();
    let to_nibbles = Nibbles::from_bytes_be(to_state_key.as_bytes()).unwrap();

    let push1 = get_push_opcode(1);
    let add = get_opcode("ADD");
    let stop = get_opcode("STOP");
    let code = [push1, 3, push1, 4, add, stop];
    let code_gas = 3 + 3 + 3;
    let code_hash = keccak(code);

    let beneficiary_account_before = AccountRlp::default();
    let sender_account_before = AccountRlp {
        nonce: 5.into(),

        balance: eth_to_wei(100_000.into()),

        ..AccountRlp::default()
    };
    let to_account_before = AccountRlp {
        code_hash,
        ..AccountRlp::default()
    };

    let state_trie_before: HashedPartialTrie = {
        let mut children = core::array::from_fn(|_| Node::Empty.into());
        children[sender_nibbles.get_nibble(0) as usize] = Node::Leaf {
            nibbles: sender_nibbles.truncate_n_nibbles_front(1),

            value: rlp::encode(&sender_account_before).to_vec(),
        }
        .into();
        children[to_nibbles.get_nibble(0) as usize] = Node::Leaf {
            nibbles: to_nibbles.truncate_n_nibbles_front(1),

            value: rlp::encode(&to_account_before).to_vec(),
        }
        .into();
        Node::Branch {
            children,
            value: vec![],
        }
    }
    .into();
    let genesis_state_trie_root = state_trie_before.hash();

    let tries_before = TrieInputs {
        state_trie: state_trie_before,
        transactions_trie: Node::Empty.into(),
        receipts_trie: Node::Empty.into(),
        storage_tries: vec![],
    };

    // Generated using a little py-evm script.
    let txn1 = hex!("f861050a8255f094a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0648242421ba02c89eb757d9deeb1f5b3859a9d4d679951ef610ac47ad4608dc142beb1b7e313a05af7e9fbab825455d36c36c7f4cfcafbeafa9a77bdff936b52afb36d4fe4bcdd");
    let txn2 = hex!("f863800a83061a8094095e7baea6a6c7c4c2dfeb977efac326af552d87830186a0801ba0ffb600e63115a7362e7811894a91d8ba4330e526f22121c994c4692035dfdfd5a06198379fcac8de3dbfac48b165df4bf88e2088f294b61efb9a65fe2281c76e16");
    let txn3 = hex!("f861800a8405f5e10094100000000000000000000000000000000000000080801ba07e09e26678ed4fac08a249ebe8ed680bf9051a5e14ad223e4b2b9d26e0208f37a05f6e3f188e3e6eab7d7d3b6568f5eac7d687b08d307d3154ccd8c87b4630509b");
    let txn4 = hex!("f866800a82520894095e7baea6a6c7c4c2dfeb977efac326af552d878711c37937e080008026a01fcd0ce88ac7600698a771f206df24b70e67981b6f107bd7c1c24ea94f113bcba00d87cc5c7afc2988e4ff200b5a0c7016b0d5498bbc692065ca983fcbbfe02555");

    let txdata_gas = 2 * 16;
    let gas_used = 21_000 + code_gas + txdata_gas;

    let value = U256::from(100u32);

    let block_metadata = BlockMetadata {
        block_beneficiary: Address::from(beneficiary),
        block_timestamp: 0x03e8.into(),
        block_number: 1.into(),
        block_difficulty: 0x020000.into(),
        block_gaslimit: 0x445566u64.into(),
        block_chain_id: 1.into(),
        block_gas_used: gas_used.into(),
        ..BlockMetadata::default()
    };

    let mut contract_code = HashMap::new();
    contract_code.insert(keccak(vec![]), vec![]);
    contract_code.insert(code_hash, code.to_vec());

    // Update trie roots after the 4 transactions.
    // State trie.
    let expected_state_trie_after: HashedPartialTrie = {
        let beneficiary_account_after = AccountRlp {
            balance: beneficiary_account_before.balance + gas_used * 10,
            ..beneficiary_account_before
        };
        let sender_account_after = AccountRlp {
            balance: sender_account_before.balance - value - gas_used * 10,
            nonce: sender_account_before.nonce + 1,
            ..sender_account_before
        };
        let to_account_after = AccountRlp {
            balance: to_account_before.balance + value,
            ..to_account_before
        };

        let mut children = core::array::from_fn(|_| Node::Empty.into());
        children[beneficiary_nibbles.get_nibble(0) as usize] = Node::Leaf {
            nibbles: beneficiary_nibbles.truncate_n_nibbles_front(1),

            value: rlp::encode(&beneficiary_account_after).to_vec(),
        }
        .into();
        children[sender_nibbles.get_nibble(0) as usize] = Node::Leaf {
            nibbles: sender_nibbles.truncate_n_nibbles_front(1),

            value: rlp::encode(&sender_account_after).to_vec(),
        }
        .into();
        children[to_nibbles.get_nibble(0) as usize] = Node::Leaf {
            nibbles: to_nibbles.truncate_n_nibbles_front(1),

            value: rlp::encode(&to_account_after).to_vec(),
        }
        .into();
        Node::Branch {
            children,
            value: vec![],
        }
    }
    .into();

    // Transactions trie.
    let mut transactions_trie: HashedPartialTrie = Node::Leaf {
        nibbles: Nibbles::from_str("0x80").unwrap(),
        value: txn1.to_vec(),
    }
    .into();
    transactions_trie.insert(Nibbles::from_str("0x01").unwrap(), txn2.to_vec());
    transactions_trie.insert(Nibbles::from_str("0x02").unwrap(), txn3.to_vec());
    transactions_trie.insert(Nibbles::from_str("0x03").unwrap(), txn4.to_vec());

    // Receipts trie.
    let mut receipts_trie = HashedPartialTrie::from(Node::Empty);
    let receipt_0 = LegacyReceiptRlp {
        status: true,
        cum_gas_used: gas_used.into(),
        bloom: [0x00; 256].to_vec().into(),
        logs: vec![],
    };
    let receipt_1 = LegacyReceiptRlp {
        status: false,
        cum_gas_used: gas_used.into(),
        bloom: [0x00; 256].to_vec().into(),
        logs: vec![],
    };
    receipts_trie.insert(
        Nibbles::from_str("0x80").unwrap(),
        rlp::encode(&receipt_0).to_vec(),
    );
    receipts_trie.insert(
        Nibbles::from_str("0x01").unwrap(),
        rlp::encode(&receipt_1).to_vec(),
    );
    receipts_trie.insert(
        Nibbles::from_str("0x02").unwrap(),
        rlp::encode(&receipt_1).to_vec(),
    );
    receipts_trie.insert(
        Nibbles::from_str("0x03").unwrap(),
        rlp::encode(&receipt_1).to_vec(),
    );

    let trie_roots_after = TrieRoots {
        state_root: expected_state_trie_after.hash(),
        transactions_root: transactions_trie.hash(),
        receipts_root: receipts_trie.hash(),
    };
    let inputs = GenerationInputs {
        signed_txns: vec![txn1.to_vec(), txn2.to_vec(), txn3.to_vec(), txn4.to_vec()],
        tries: tries_before,
        trie_roots_after,
        genesis_state_trie_root,
        contract_code,
        block_metadata: block_metadata.clone(),
        addresses: vec![],
        block_bloom_before: [0.into(); 8],
        gas_used_before: 0.into(),
        gas_used_after: gas_used.into(),
        txn_number_before: 0.into(),
        block_bloom_after: [0.into(); 8],
        block_hashes: BlockHashes {
            prev_hashes: vec![H256::default(); 256],
            cur_hash: H256::default(),
        },
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
