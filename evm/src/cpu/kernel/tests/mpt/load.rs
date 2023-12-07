use std::str::FromStr;

use anyhow::{anyhow, Result};
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::HashedPartialTrie;
use ethereum_types::{BigEndianHash, H256, U256};
use hex_literal::hex;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::constants::trie_type::PartialTrieType;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::account_code::initialize_mpts;
use crate::cpu::kernel::tests::mpt::{extension_to_leaf, test_account_1, test_account_1_rlp};
use crate::generation::TrieInputs;
use crate::Node;

#[test]
fn load_all_mpts_empty() -> Result<()> {
    let trie_inputs = TrieInputs {
        state_trie: Default::default(),
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };

    let initial_stack = vec![];
    let mut interpreter = Interpreter::new_with_kernel(0, initial_stack);
    initialize_mpts(&mut interpreter, &trie_inputs);
    assert_eq!(interpreter.stack(), vec![]);

    // We need to have the first element in `TrieData` be 0.
    assert_eq!(interpreter.get_trie_data(), vec![0.into()]);

    assert_eq!(
        interpreter.get_global_metadata_field(GlobalMetadata::StateTrieRoot),
        0.into()
    );
    assert_eq!(
        interpreter.get_global_metadata_field(GlobalMetadata::TransactionTrieRoot),
        0.into()
    );
    assert_eq!(
        interpreter.get_global_metadata_field(GlobalMetadata::ReceiptTrieRoot),
        0.into()
    );

    Ok(())
}

#[test]
fn load_all_mpts_leaf() -> Result<()> {
    let trie_inputs = TrieInputs {
        state_trie: Node::Leaf {
            nibbles: 0xABC_u64.into(),
            value: test_account_1_rlp(),
        }
        .into(),
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };

    let initial_stack = vec![];
    let mut interpreter = Interpreter::new_with_kernel(0, initial_stack);
    initialize_mpts(&mut interpreter, &trie_inputs);
    assert_eq!(interpreter.stack(), vec![]);

    let type_leaf = U256::from(PartialTrieType::Leaf as u32);
    assert_eq!(
        interpreter.get_trie_data(),
        vec![
            0.into(),
            type_leaf,
            3.into(),
            0xABC.into(),
            5.into(), // value ptr
            test_account_1().nonce,
            test_account_1().balance,
            9.into(), // pointer to storage trie root
            test_account_1().code_hash.into_uint(),
            // These last two elements encode the storage trie, which is a hash node.
            (PartialTrieType::Hash as u32).into(),
            test_account_1().storage_root.into_uint(),
        ]
    );

    assert_eq!(
        interpreter.get_global_metadata_field(GlobalMetadata::TransactionTrieRoot),
        0.into()
    );
    assert_eq!(
        interpreter.get_global_metadata_field(GlobalMetadata::ReceiptTrieRoot),
        0.into()
    );

    Ok(())
}

#[test]
fn load_all_mpts_hash() -> Result<()> {
    let hash = H256::random();
    let trie_inputs = TrieInputs {
        state_trie: Node::Hash(hash).into(),
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };

    let initial_stack = vec![];
    let mut interpreter = Interpreter::new_with_kernel(0, initial_stack);
    initialize_mpts(&mut interpreter, &trie_inputs);
    assert_eq!(interpreter.stack(), vec![]);

    let type_hash = U256::from(PartialTrieType::Hash as u32);
    assert_eq!(
        interpreter.get_trie_data(),
        vec![0.into(), type_hash, hash.into_uint(),]
    );

    assert_eq!(
        interpreter.get_global_metadata_field(GlobalMetadata::TransactionTrieRoot),
        0.into()
    );
    assert_eq!(
        interpreter.get_global_metadata_field(GlobalMetadata::ReceiptTrieRoot),
        0.into()
    );

    Ok(())
}

#[test]
fn load_all_mpts_empty_branch() -> Result<()> {
    let children = core::array::from_fn(|_| Node::Empty.into());
    let state_trie = Node::Branch {
        children,
        value: vec![],
    }
    .into();
    let trie_inputs = TrieInputs {
        state_trie,
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };

    let initial_stack = vec![];
    let mut interpreter = Interpreter::new_with_kernel(0, initial_stack);
    initialize_mpts(&mut interpreter, &trie_inputs);
    assert_eq!(interpreter.stack(), vec![]);

    let type_branch = U256::from(PartialTrieType::Branch as u32);
    assert_eq!(
        interpreter.get_trie_data(),
        vec![
            0.into(), // First address is unused, so that 0 can be treated as a null pointer.
            type_branch,
            0.into(), // child 0
            0.into(), // ...
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(),
            0.into(), // child 16
            0.into(), // value_ptr
        ]
    );

    assert_eq!(
        interpreter.get_global_metadata_field(GlobalMetadata::TransactionTrieRoot),
        0.into()
    );
    assert_eq!(
        interpreter.get_global_metadata_field(GlobalMetadata::ReceiptTrieRoot),
        0.into()
    );

    Ok(())
}

#[test]
fn load_all_mpts_ext_to_leaf() -> Result<()> {
    let trie_inputs = TrieInputs {
        state_trie: extension_to_leaf(test_account_1_rlp()),
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };

    let initial_stack = vec![];
    let mut interpreter = Interpreter::new_with_kernel(0, initial_stack);
    initialize_mpts(&mut interpreter, &trie_inputs);
    assert_eq!(interpreter.stack(), vec![]);

    let type_extension = U256::from(PartialTrieType::Extension as u32);
    let type_leaf = U256::from(PartialTrieType::Leaf as u32);
    assert_eq!(
        interpreter.get_trie_data(),
        vec![
            0.into(), // First address is unused, so that 0 can be treated as a null pointer.
            type_extension,
            3.into(),     // 3 nibbles
            0xABC.into(), // key part
            5.into(),     // Pointer to the leaf node immediately below.
            type_leaf,
            3.into(),     // 3 nibbles
            0xDEF.into(), // key part
            9.into(),     // value pointer
            test_account_1().nonce,
            test_account_1().balance,
            13.into(), // pointer to storage trie root
            test_account_1().code_hash.into_uint(),
            // These last two elements encode the storage trie, which is a hash node.
            (PartialTrieType::Hash as u32).into(),
            test_account_1().storage_root.into_uint(),
        ]
    );

    Ok(())
}

#[test]
fn load_mpt_txn_trie() -> Result<()> {
    let txn = hex!("f860010a830186a094095e7baea6a6c7c4c2dfeb977efac326af552e89808025a04a223955b0bd3827e3740a9a427d0ea43beb5bafa44a0204bf0a3306c8219f7ba0502c32d78f233e9e7ce9f5df3b576556d5d49731e0678fd5a068cdf359557b5b").to_vec();

    let trie_inputs = TrieInputs {
        state_trie: Default::default(),
        transactions_trie: HashedPartialTrie::from(Node::Leaf {
            nibbles: Nibbles::from_str("0x80").unwrap(),
            value: txn.clone(),
        }),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };

    let initial_stack = vec![];
    let mut interpreter = Interpreter::new_with_kernel(0, initial_stack);
    initialize_mpts(&mut interpreter, &trie_inputs);
    assert_eq!(interpreter.stack(), vec![]);

    let mut expected_trie_data = vec![
        0.into(),
        U256::from(PartialTrieType::Leaf as u32),
        2.into(),
        128.into(), // Nibble
        5.into(),   // value_ptr
        txn.len().into(),
    ];
    expected_trie_data.extend(txn.into_iter().map(U256::from));
    let trie_data = interpreter.get_trie_data();

    assert_eq!(trie_data, expected_trie_data);

    Ok(())
}
