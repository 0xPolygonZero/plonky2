use anyhow::Result;
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{BigEndianHash, H256};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::mpt::{nibbles_64, test_account_1_rlp, test_account_2};
use crate::generation::mpt::{all_mpt_prover_inputs_reversed, AccountRlp};
use crate::generation::TrieInputs;
use crate::Node;

#[test]
fn mpt_delete_empty() -> Result<()> {
    test_state_trie(Default::default(), nibbles_64(0xABC), test_account_2())
}

#[test]
fn mpt_delete_leaf_nonoverlapping_keys() -> Result<()> {
    let state_trie = Node::Leaf {
        nibbles: nibbles_64(0xABC),
        value: test_account_1_rlp(),
    }
    .into();
    test_state_trie(state_trie, nibbles_64(0x123), test_account_2())
}

#[test]
fn mpt_delete_leaf_overlapping_keys() -> Result<()> {
    let state_trie = Node::Leaf {
        nibbles: nibbles_64(0xABC),
        value: test_account_1_rlp(),
    }
    .into();
    test_state_trie(state_trie, nibbles_64(0xADE), test_account_2())
}

/// Note: The account's storage_root is ignored, as we can't insert a new storage_root without the
/// accompanying trie data. An empty trie's storage_root is used instead.
fn test_state_trie(
    mut state_trie: HashedPartialTrie,
    k: Nibbles,
    mut account: AccountRlp,
) -> Result<()> {
    assert_eq!(k.count, 64);

    // Ignore any storage_root; see documentation note.
    account.storage_root = HashedPartialTrie::from(Node::Empty).hash();

    let trie_inputs = TrieInputs {
        state_trie: state_trie.clone(),
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };
    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];
    let mpt_insert_state_trie = KERNEL.global_labels["mpt_insert_state_trie"];
    let mpt_delete = KERNEL.global_labels["mpt_delete"];
    let mpt_hash_state_trie = KERNEL.global_labels["mpt_hash_state_trie"];
    let wat = KERNEL.global_labels["wattt"];
    let yo = KERNEL.global_labels["yoo"];

    let initial_stack = vec![0xDEADBEEFu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(load_all_mpts, initial_stack);
    interpreter.generation_state.mpt_prover_inputs = all_mpt_prover_inputs_reversed(&trie_inputs);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    // Next, execute mpt_insert_state_trie.
    interpreter.generation_state.registers.program_counter = mpt_insert_state_trie;
    let trie_data = interpreter.get_trie_data_mut();
    if trie_data.is_empty() {
        // In the assembly we skip over 0, knowing trie_data[0] = 0 by default.
        // Since we don't explicitly set it to 0, we need to do so here.
        trie_data.push(0.into());
    }
    let value_ptr = trie_data.len();
    trie_data.push(account.nonce);
    trie_data.push(account.balance);
    // In memory, storage_root gets interpreted as a pointer to a storage trie,
    // so we have to ensure the pointer is valid. It's easiest to set it to 0,
    // which works as an empty node, since trie_data[0] = 0 = MPT_TYPE_EMPTY.
    trie_data.push(H256::zero().into_uint());
    trie_data.push(account.code_hash.into_uint());
    let trie_data_len = trie_data.len().into();
    interpreter.set_global_metadata_field(GlobalMetadata::TrieDataSize, trie_data_len);
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.push(value_ptr.into()); // value_ptr
    interpreter.push(k.packed); // key
    interpreter.run()?;
    assert_eq!(
        interpreter.stack().len(),
        0,
        "Expected empty stack after insert, found {:?}",
        interpreter.stack()
    );
    let state_trie_ptr = interpreter.get_global_metadata_field(GlobalMetadata::StateTrieRoot);
    // dbg!(state_trie_ptr);
    // dbg!(interpreter.get_trie_data());
    interpreter.generation_state.registers.program_counter = mpt_delete;
    let ya = KERNEL.global_labels["ya"];
    let bro = KERNEL.global_labels["bro"];
    interpreter.debug_offsets.push(wat);
    interpreter.debug_offsets.push(yo);
    interpreter.debug_offsets.push(ya);
    interpreter.debug_offsets.push(bro);
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.push(k.packed);
    interpreter.push(64.into());
    interpreter.push(state_trie_ptr);
    interpreter.run()?;
    dbg!(interpreter.stack());
    let state_trie_ptr = interpreter.pop();
    interpreter.set_global_metadata_field(GlobalMetadata::StateTrieRoot, state_trie_ptr);

    // Now, execute mpt_hash_state_trie.
    interpreter.generation_state.registers.program_counter = mpt_hash_state_trie;
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.run()?;
    let state_trie_hash = H256::from_uint(&interpreter.pop());
    let expected_state_trie_hash = state_trie.hash();
    assert_eq!(state_trie_hash, expected_state_trie_hash);

    Ok(())
}
