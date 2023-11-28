use std::collections::HashMap;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, Node, PartialTrie};
use ethereum_types::{Address, H256, U256};
use hex_literal::hex;
use keccak_hash::keccak;
use smt_utils::account::Account;
use smt_utils::smt::{hash_serialize_state, Smt};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::context_metadata::ContextMetadata;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::generation::mpt::{
    all_mpt_prover_inputs_reversed, state_smt_prover_inputs_reversed, LegacyReceiptRlp,
};
use crate::generation::rlp::all_rlp_prover_inputs_reversed;
use crate::generation::TrieInputs;
use crate::memory::segments::Segment;
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
    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];

    interpreter.generation_state.registers.program_counter = load_all_mpts;
    interpreter.push(0xDEADBEEFu32.into());

    interpreter.generation_state.state_smt_prover_inputs =
        state_smt_prover_inputs_reversed(&trie_inputs);
    interpreter.generation_state.mpt_prover_inputs =
        all_mpt_prover_inputs_reversed(&trie_inputs).expect("Invalid MPT data.");
    interpreter.run().expect("MPT loading failed.");
    assert_eq!(interpreter.stack(), vec![]);

    // Set necessary `GlobalMetadata`.
    let global_metadata_to_set = [
        (
            GlobalMetadata::StateTrieRootDigestBefore,
            h2u(hash_serialize_state(&trie_inputs.state_smt)),
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
    let to_key = keccak(to);

    let beneficiary_bits = beneficiary_state_key.into();
    let sender_bits = sender_state_key.into();
    let to_bits = to_key.into();

    let code = [0x60, 0x01, 0x60, 0x01, 0x01, 0x60, 0x00, 0x55, 0x00];
    let code_hash = keccak(code);

    let mut contract_code = HashMap::new();
    contract_code.insert(keccak(vec![]), vec![]);
    contract_code.insert(code_hash, code.to_vec());

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

    let mut state_trie_before = Smt::empty();
    state_trie_before.insert(beneficiary_bits, beneficiary_account_before.into());
    state_trie_before.insert(sender_bits, sender_account_before.into());
    state_trie_before.insert(to_bits, to_account_before.into());

    let tries_before = TrieInputs {
        state_smt: state_trie_before.serialize(),
        transactions_trie: Node::Empty.into(),
        receipts_trie: Node::Empty.into(),
    };

    let txn = hex!("f863800a83061a8094095e7baea6a6c7c4c2dfeb977efac326af552d87830186a0801ba0ffb600e63115a7362e7811894a91d8ba4330e526f22121c994c4692035dfdfd5a06198379fcac8de3dbfac48b165df4bf88e2088f294b61efb9a65fe2281c76e16");

    let initial_stack = vec![];
    let mut interpreter = Interpreter::new_with_kernel(0, initial_stack);

    prepare_interpreter(&mut interpreter, tries_before.clone(), &txn, contract_code);
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

        let mut expected_state_trie_after = Smt::empty();
        expected_state_trie_after.insert(beneficiary_bits, beneficiary_account_after.into());
        expected_state_trie_after.insert(sender_bits, sender_account_after.into());
        expected_state_trie_after.insert(to_bits, to_account_after.into());
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
        state_root: expected_state_trie_after.root,
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
    interpreter.generation_state.memory.contexts[0].segments[Segment::ContextMetadata as usize]
        .set(ContextMetadata::GasLimit as usize, 1_000_000.into());
    interpreter.set_is_kernel(true);
    interpreter.run().expect("Proving add11 failed.");
}
