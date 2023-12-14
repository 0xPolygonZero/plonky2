use std::collections::HashMap;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, Node, PartialTrie};
use ethereum_types::{Address, H256, U256};
use hex_literal::hex;
use keccak_hash::keccak;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::context_metadata::ContextMetadata;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::account_code::initialize_mpts;
use crate::generation::mpt::{AccountRlp, LegacyReceiptRlp};
use crate::generation::rlp::all_rlp_prover_inputs_reversed;
use crate::generation::TrieInputs;
use crate::memory::segments::{Segment, SEGMENT_SCALING_FACTOR};
use crate::proof::TrieRoots;
use crate::util::h2u;

// Stolen from `tests/mpt/insert.rs`
// Prepare the interpreter by loading the initial MPTs and
// by setting all `GlobalMetadata` and necessary code into memory.
fn prepare_interpreter(
    interpreter: &mut Interpreter,
    trie_inputs: TrieInputs,
    transaction: &[u8],
    contract_code: HashMap<H256, Vec<u8>>,
) {
    initialize_mpts(interpreter, &trie_inputs);
    assert_eq!(interpreter.stack(), vec![]);

    // Set necessary `GlobalMetadata`.
    let global_metadata_to_set = [
        (
            GlobalMetadata::StateTrieRootDigestBefore,
            h2u(trie_inputs.state_trie.hash()),
        ),
        (
            GlobalMetadata::TransactionTrieRootDigestBefore,
            h2u(trie_inputs.transactions_trie.hash()),
        ),
        (
            GlobalMetadata::ReceiptTrieRootDigestBefore,
            h2u(trie_inputs.receipts_trie.hash()),
        ),
        (GlobalMetadata::TxnNumberAfter, 1.into()),
        (GlobalMetadata::BlockGasUsedAfter, 0xa868u64.into()),
        (GlobalMetadata::BlockGasLimit, 1_000_000.into()),
        (GlobalMetadata::BlockBaseFee, 10.into()),
        (
            GlobalMetadata::BlockBeneficiary,
            U256::from_big_endian(
                &Address::from(hex!("2adc25665018aa1fe0e6bc666dac8fc2697ff9ba")).0,
            ),
        ),
    ];

    interpreter.set_global_metadata_multi_fields(&global_metadata_to_set);

    // Set contract code and transaction.
    interpreter.generation_state.inputs.contract_code = contract_code;

    interpreter.generation_state.inputs.signed_txn = Some(transaction.to_vec());
    let rlp_prover_inputs = all_rlp_prover_inputs_reversed(transaction);
    interpreter.generation_state.rlp_prover_inputs = rlp_prover_inputs;
}

#[test]
fn test_add11_yml() {
    let beneficiary = hex!("2adc25665018aa1fe0e6bc666dac8fc2697ff9ba");
    let sender = hex!("a94f5374fce5edbc8e2a8697c15331677e6ebf0b");
    let to = hex!("095e7baea6a6c7c4c2dfeb977efac326af552d87");

    let beneficiary_state_key = keccak(beneficiary);
    let sender_state_key = keccak(sender);
    let to_hashed = keccak(to);

    let beneficiary_nibbles = Nibbles::from_bytes_be(beneficiary_state_key.as_bytes()).unwrap();
    let sender_nibbles = Nibbles::from_bytes_be(sender_state_key.as_bytes()).unwrap();
    let to_nibbles = Nibbles::from_bytes_be(to_hashed.as_bytes()).unwrap();

    let code = [0x60, 0x01, 0x60, 0x01, 0x01, 0x60, 0x00, 0x55, 0x00];
    let code_hash = keccak(code);

    let mut contract_code = HashMap::new();
    contract_code.insert(keccak(vec![]), vec![]);
    contract_code.insert(code_hash, code.to_vec());

    let beneficiary_account_before = AccountRlp {
        nonce: 1.into(),
        ..AccountRlp::default()
    };
    let sender_account_before = AccountRlp {
        balance: 0x0de0b6b3a7640000u64.into(),
        ..AccountRlp::default()
    };
    let to_account_before = AccountRlp {
        balance: 0x0de0b6b3a7640000u64.into(),
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

    let txn = hex!("f863800a83061a8094095e7baea6a6c7c4c2dfeb977efac326af552d87830186a0801ba0ffb600e63115a7362e7811894a91d8ba4330e526f22121c994c4692035dfdfd5a06198379fcac8de3dbfac48b165df4bf88e2088f294b61efb9a65fe2281c76e16");

    let initial_stack = vec![];
    let mut interpreter = Interpreter::new_with_kernel(0, initial_stack);

    prepare_interpreter(&mut interpreter, tries_before.clone(), &txn, contract_code);
    let expected_state_trie_after = {
        let beneficiary_account_after = AccountRlp {
            nonce: 1.into(),
            ..AccountRlp::default()
        };
        let sender_account_after = AccountRlp {
            balance: 0xde0b6b3a75be550u64.into(),
            nonce: 1.into(),
            ..AccountRlp::default()
        };
        let to_account_after = AccountRlp {
            balance: 0xde0b6b3a76586a0u64.into(),
            code_hash,
            // Storage map: { 0 => 2 }
            storage_root: HashedPartialTrie::from(Node::Leaf {
                nibbles: Nibbles::from_h256_be(keccak([0u8; 32])),
                value: vec![2],
            })
            .hash(),
            ..AccountRlp::default()
        };

        let mut expected_state_trie_after = HashedPartialTrie::from(Node::Empty);
        expected_state_trie_after.insert(
            beneficiary_nibbles,
            rlp::encode(&beneficiary_account_after).to_vec(),
        );
        expected_state_trie_after
            .insert(sender_nibbles, rlp::encode(&sender_account_after).to_vec());
        expected_state_trie_after.insert(to_nibbles, rlp::encode(&to_account_after).to_vec());
        expected_state_trie_after
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
        state_root: expected_state_trie_after.hash(),
        transactions_root: transactions_trie.hash(),
        receipts_root: receipts_trie.hash(),
    };

    // Set trie roots after the transaction was executed.
    let metadata_to_set = [
        (
            GlobalMetadata::StateTrieRootDigestAfter,
            h2u(trie_roots_after.state_root),
        ),
        (
            GlobalMetadata::TransactionTrieRootDigestAfter,
            h2u(trie_roots_after.transactions_root),
        ),
        (
            GlobalMetadata::ReceiptTrieRootDigestAfter,
            h2u(trie_roots_after.receipts_root),
        ),
    ];
    interpreter.set_global_metadata_multi_fields(&metadata_to_set);

    let route_txn_label = KERNEL.global_labels["hash_initial_tries"];
    // Switch context and initialize memory with the data we need for the tests.
    interpreter.generation_state.registers.program_counter = route_txn_label;
    interpreter.set_context_metadata_field(0, ContextMetadata::GasLimit, 1_000_000.into());
    interpreter.set_is_kernel(true);
    interpreter.run().expect("Proving add11 failed.");
}
