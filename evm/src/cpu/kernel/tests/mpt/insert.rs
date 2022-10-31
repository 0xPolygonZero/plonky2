use anyhow::Result;
use eth_trie_utils::partial_trie::{Nibbles, PartialTrie};
use ethereum_types::{BigEndianHash, H256};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::mpt::{
    nibbles_64, nibbles_count, test_account_1_rlp, test_account_2,
};
use crate::generation::mpt::{all_mpt_prover_inputs_reversed, AccountRlp};
use crate::generation::TrieInputs;

#[test]
fn mpt_insert_empty() -> Result<()> {
    test_state_trie(Default::default(), nibbles_64(0xABC), test_account_2())
}

#[test]
fn mpt_insert_leaf_identical_keys() -> Result<()> {
    let key = nibbles_64(0xABC);
    let state_trie = PartialTrie::Leaf {
        nibbles: key,
        value: test_account_1_rlp(),
    };
    test_state_trie(state_trie, key, test_account_2())
}

#[test]
fn mpt_insert_leaf_nonoverlapping_keys() -> Result<()> {
    let state_trie = PartialTrie::Leaf {
        nibbles: nibbles_64(0xABC),
        value: test_account_1_rlp(),
    };
    test_state_trie(state_trie, nibbles_64(0x123), test_account_2())
}

#[test]
fn mpt_insert_leaf_overlapping_keys() -> Result<()> {
    let state_trie = PartialTrie::Leaf {
        nibbles: nibbles_64(0xABC),
        value: test_account_1_rlp(),
    };
    test_state_trie(state_trie, nibbles_64(0xADE), test_account_2())
}

#[test]
#[ignore] // TODO: Not valid for state trie, all keys have same len.
fn mpt_insert_leaf_insert_key_extends_leaf_key() -> Result<()> {
    let state_trie = PartialTrie::Leaf {
        nibbles: 0xABC_u64.into(),
        value: test_account_1_rlp(),
    };
    test_state_trie(state_trie, nibbles_64(0xABCDE), test_account_2())
}

#[test]
#[ignore] // TODO: Not valid for state trie, all keys have same len.
fn mpt_insert_leaf_leaf_key_extends_insert_key() -> Result<()> {
    let state_trie = PartialTrie::Leaf {
        nibbles: 0xABCDE_u64.into(),
        value: test_account_1_rlp(),
    };
    test_state_trie(state_trie, nibbles_64(0xABC), test_account_2())
}

#[test]
fn mpt_insert_branch_replacing_empty_child() -> Result<()> {
    let children = std::array::from_fn(|_| PartialTrie::Empty.into());
    let state_trie = PartialTrie::Branch {
        children,
        value: vec![],
    };

    test_state_trie(state_trie, nibbles_64(0xABC), test_account_2())
}

#[test]
// TODO: Not a valid test because branches state trie cannot have branch values.
// We should change it to use a different trie.
#[ignore]
fn mpt_insert_extension_nonoverlapping_keys() -> Result<()> {
    // Existing keys are 0xABC, 0xABCDEF; inserted key is 0x12345.
    let mut children = std::array::from_fn(|_| PartialTrie::Empty.into());
    children[0xD] = PartialTrie::Leaf {
        nibbles: 0xEF_u64.into(),
        value: test_account_1_rlp(),
    }
    .into();
    let state_trie = PartialTrie::Extension {
        nibbles: 0xABC_u64.into(),
        child: PartialTrie::Branch {
            children,
            value: test_account_1_rlp(),
        }
        .into(),
    };
    test_state_trie(state_trie, nibbles_64(0x12345), test_account_2())
}

#[test]
// TODO: Not a valid test because branches state trie cannot have branch values.
// We should change it to use a different trie.
#[ignore]
fn mpt_insert_extension_insert_key_extends_node_key() -> Result<()> {
    // Existing keys are 0xA, 0xABCD; inserted key is 0xABCDEF.
    let mut children = std::array::from_fn(|_| PartialTrie::Empty.into());
    children[0xB] = PartialTrie::Leaf {
        nibbles: 0xCD_u64.into(),
        value: test_account_1_rlp(),
    }
    .into();
    let state_trie = PartialTrie::Extension {
        nibbles: 0xA_u64.into(),
        child: PartialTrie::Branch {
            children,
            value: test_account_1_rlp(),
        }
        .into(),
    };
    test_state_trie(state_trie, nibbles_64(0xABCDEF), test_account_2())
}

#[test]
fn mpt_insert_branch_to_leaf_same_key() -> Result<()> {
    let leaf = PartialTrie::Leaf {
        nibbles: nibbles_count(0xBCD, 63),
        value: test_account_1_rlp(),
    }
    .into();

    let mut children = std::array::from_fn(|_| PartialTrie::Empty.into());
    children[0] = leaf;
    let state_trie = PartialTrie::Branch {
        children,
        value: vec![],
    };

    test_state_trie(state_trie, nibbles_64(0xABCD), test_account_2())
}

/// Note: The account's storage_root is ignored, as we can't insert a new storage_root without the
/// accompanying trie data. An empty trie's storage_root is used instead.
fn test_state_trie(mut state_trie: PartialTrie, k: Nibbles, mut account: AccountRlp) -> Result<()> {
    assert_eq!(k.count, 64);

    // Ignore any storage_root; see documentation note.
    account.storage_root = PartialTrie::Empty.calc_hash();

    let trie_inputs = TrieInputs {
        state_trie: state_trie.clone(),
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };
    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];
    let mpt_insert_state_trie = KERNEL.global_labels["mpt_insert_state_trie"];
    let mpt_hash_state_trie = KERNEL.global_labels["mpt_hash_state_trie"];

    let initial_stack = vec![0xDEADBEEFu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(load_all_mpts, initial_stack);
    interpreter.generation_state.mpt_prover_inputs = all_mpt_prover_inputs_reversed(&trie_inputs);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    // Next, execute mpt_insert_state_trie.
    interpreter.offset = mpt_insert_state_trie;
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

    // Now, execute mpt_hash_state_trie.
    interpreter.offset = mpt_hash_state_trie;
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.run()?;

    assert_eq!(
        interpreter.stack().len(),
        1,
        "Expected 1 item on stack after hashing, found {:?}",
        interpreter.stack()
    );
    let hash = H256::from_uint(&interpreter.stack()[0]);

    state_trie.insert(k, rlp::encode(&account).to_vec());
    let expected_state_trie_hash = state_trie.calc_hash();
    assert_eq!(hash, expected_state_trie_hash);

    Ok(())
}
