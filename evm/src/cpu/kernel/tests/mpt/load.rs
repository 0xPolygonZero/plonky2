use anyhow::Result;
use eth_trie_utils::partial_trie::PartialTrie;
use ethereum_types::{BigEndianHash, H256, U256};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::constants::trie_type::PartialTrieType;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::mpt::{extension_to_leaf, test_account_1, test_account_1_rlp};
use crate::generation::mpt::all_mpt_prover_inputs_reversed;
use crate::generation::TrieInputs;

#[test]
fn load_all_mpts_empty() -> Result<()> {
    let trie_inputs = TrieInputs {
        state_trie: Default::default(),
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };

    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];

    let initial_stack = vec![0xDEADBEEFu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(load_all_mpts, initial_stack);
    interpreter.generation_state.mpt_prover_inputs = all_mpt_prover_inputs_reversed(&trie_inputs);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    assert_eq!(interpreter.get_trie_data(), vec![]);

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
        state_trie: PartialTrie::Leaf {
            nibbles: 0xABC_u64.into(),
            value: test_account_1_rlp(),
        },
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };

    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];

    let initial_stack = vec![0xDEADBEEFu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(load_all_mpts, initial_stack);
    interpreter.generation_state.mpt_prover_inputs = all_mpt_prover_inputs_reversed(&trie_inputs);
    interpreter.run()?;
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
        state_trie: PartialTrie::Hash(hash),
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };

    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];

    let initial_stack = vec![0xDEADBEEFu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(load_all_mpts, initial_stack);
    interpreter.generation_state.mpt_prover_inputs = all_mpt_prover_inputs_reversed(&trie_inputs);
    interpreter.run()?;
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
    let children = std::array::from_fn(|_| PartialTrie::Empty.into());
    let state_trie = PartialTrie::Branch {
        children,
        value: vec![],
    };
    let trie_inputs = TrieInputs {
        state_trie,
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };

    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];

    let initial_stack = vec![0xDEADBEEFu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(load_all_mpts, initial_stack);
    interpreter.generation_state.mpt_prover_inputs = all_mpt_prover_inputs_reversed(&trie_inputs);
    interpreter.run()?;
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

    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];

    let initial_stack = vec![0xDEADBEEFu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(load_all_mpts, initial_stack);
    interpreter.generation_state.mpt_prover_inputs = all_mpt_prover_inputs_reversed(&trie_inputs);
    interpreter.run()?;
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
