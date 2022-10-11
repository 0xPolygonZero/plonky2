use anyhow::Result;
use eth_trie_utils::partial_trie::{Nibbles, PartialTrie};
use eth_trie_utils::trie_builder::InsertEntry;
use ethereum_types::{BigEndianHash, H256};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::mpt::{test_account_1_rlp, test_account_2_rlp};
use crate::generation::mpt::{all_mpt_prover_inputs_reversed, AccountRlp};
use crate::generation::TrieInputs;

#[test]
fn mpt_insert_empty() -> Result<()> {
    let insert = InsertEntry {
        nibbles: Nibbles {
            count: 3,
            packed: 0xABC.into(),
        },
        v: test_account_2_rlp(),
    };
    test_state_trie(Default::default(), insert)
}

#[test]
fn mpt_insert_leaf_identical_keys() -> Result<()> {
    let key = Nibbles {
        count: 3,
        packed: 0xABC.into(),
    };
    let state_trie = PartialTrie::Leaf {
        nibbles: key,
        value: test_account_1_rlp(),
    };
    let insert = InsertEntry {
        nibbles: key,
        v: test_account_2_rlp(),
    };
    test_state_trie(state_trie, insert)
}

#[test]
fn mpt_insert_leaf_nonoverlapping_keys() -> Result<()> {
    let state_trie = PartialTrie::Leaf {
        nibbles: Nibbles {
            count: 3,
            packed: 0xABC.into(),
        },
        value: test_account_1_rlp(),
    };
    let insert = InsertEntry {
        nibbles: Nibbles {
            count: 3,
            packed: 0x123.into(),
        },
        v: test_account_2_rlp(),
    };
    test_state_trie(state_trie, insert)
}

#[test]
fn mpt_insert_leaf_overlapping_keys() -> Result<()> {
    let state_trie = PartialTrie::Leaf {
        nibbles: Nibbles {
            count: 3,
            packed: 0xABC.into(),
        },
        value: test_account_1_rlp(),
    };
    let insert = InsertEntry {
        nibbles: Nibbles {
            count: 3,
            packed: 0xADE.into(),
        },
        v: test_account_2_rlp(),
    };
    test_state_trie(state_trie, insert)
}

#[test]
fn mpt_insert_leaf_insert_key_extends_leaf_key() -> Result<()> {
    let state_trie = PartialTrie::Leaf {
        nibbles: Nibbles {
            count: 3,
            packed: 0xABC.into(),
        },
        value: test_account_1_rlp(),
    };
    let insert = InsertEntry {
        nibbles: Nibbles {
            count: 5,
            packed: 0xABCDE.into(),
        },
        v: test_account_2_rlp(),
    };
    test_state_trie(state_trie, insert)
}

#[test]
fn mpt_insert_leaf_leaf_key_extends_insert_key() -> Result<()> {
    let state_trie = PartialTrie::Leaf {
        nibbles: Nibbles {
            count: 5,
            packed: 0xABCDE.into(),
        },
        value: test_account_1_rlp(),
    };
    let insert = InsertEntry {
        nibbles: Nibbles {
            count: 3,
            packed: 0xABC.into(),
        },
        v: test_account_2_rlp(),
    };
    test_state_trie(state_trie, insert)
}

#[test]
fn mpt_insert_branch_replacing_empty_child() -> Result<()> {
    let children = std::array::from_fn(|_| Box::new(PartialTrie::Empty));
    let state_trie = PartialTrie::Branch {
        children,
        value: vec![],
    };

    let insert = InsertEntry {
        nibbles: Nibbles {
            count: 3,
            packed: 0xABC.into(),
        },
        v: test_account_2_rlp(),
    };

    test_state_trie(state_trie, insert)
}

#[test]
fn mpt_insert_extension_nonoverlapping_keys() -> Result<()> {
    // Existing keys are 0xABC, 0xABCDEF; inserted key is 0x12345.
    let mut children = std::array::from_fn(|_| Box::new(PartialTrie::Empty));
    children[0xD] = Box::new(PartialTrie::Leaf {
        nibbles: Nibbles {
            count: 2,
            packed: 0xEF.into(),
        },
        value: test_account_1_rlp(),
    });
    let state_trie = PartialTrie::Extension {
        nibbles: Nibbles {
            count: 3,
            packed: 0xABC.into(),
        },
        child: Box::new(PartialTrie::Branch {
            children,
            value: test_account_1_rlp(),
        }),
    };
    let insert = InsertEntry {
        nibbles: Nibbles {
            count: 5,
            packed: 0x12345.into(),
        },
        v: test_account_2_rlp(),
    };
    test_state_trie(state_trie, insert)
}

#[test]
fn mpt_insert_extension_insert_key_extends_node_key() -> Result<()> {
    // Existing keys are 0xA, 0xABCD; inserted key is 0xABCDEF.
    let mut children = std::array::from_fn(|_| Box::new(PartialTrie::Empty));
    children[0xB] = Box::new(PartialTrie::Leaf {
        nibbles: Nibbles {
            count: 2,
            packed: 0xCD.into(),
        },
        value: test_account_1_rlp(),
    });
    let state_trie = PartialTrie::Extension {
        nibbles: Nibbles {
            count: 1,
            packed: 0xA.into(),
        },
        child: Box::new(PartialTrie::Branch {
            children,
            value: test_account_1_rlp(),
        }),
    };
    let insert = InsertEntry {
        nibbles: Nibbles {
            count: 6,
            packed: 0xABCDEF.into(),
        },
        v: test_account_2_rlp(),
    };
    test_state_trie(state_trie, insert)
}

#[test]
fn mpt_insert_branch_to_leaf_same_key() -> Result<()> {
    let leaf = PartialTrie::Leaf {
        nibbles: Nibbles {
            count: 3,
            packed: 0xBCD.into(),
        },
        value: test_account_1_rlp(),
    };
    let mut children = std::array::from_fn(|_| Box::new(PartialTrie::Empty));
    children[0xA] = Box::new(leaf);
    let state_trie = PartialTrie::Branch {
        children,
        value: vec![],
    };

    let insert = InsertEntry {
        nibbles: Nibbles {
            count: 4,
            packed: 0xABCD.into(),
        },
        v: test_account_2_rlp(),
    };

    test_state_trie(state_trie, insert)
}

fn test_state_trie(state_trie: PartialTrie, insert: InsertEntry) -> Result<()> {
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
    let account: AccountRlp = rlp::decode(&insert.v).expect("Decoding failed");
    let account_data = account.to_vec();
    trie_data.push(account_data.len().into());
    trie_data.extend(account_data);
    let trie_data_len = trie_data.len().into();
    interpreter.set_global_metadata_field(GlobalMetadata::TrieDataSize, trie_data_len);
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.push(value_ptr.into()); // value_ptr
    interpreter.push(insert.nibbles.packed); // key
    interpreter.push(insert.nibbles.count.into()); // num_nibbles

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

    let updated_trie = apply_insert(state_trie, insert);
    let expected_state_trie_hash = updated_trie.calc_hash();
    assert_eq!(hash, expected_state_trie_hash);

    Ok(())
}

fn apply_insert(trie: PartialTrie, insert: InsertEntry) -> PartialTrie {
    let mut trie = Box::new(trie);
    if let Some(updated_trie) = PartialTrie::insert_into_trie(&mut trie, insert) {
        *updated_trie
    } else {
        *trie
    }
}
