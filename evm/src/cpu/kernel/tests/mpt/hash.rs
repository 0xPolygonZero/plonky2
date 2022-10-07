use anyhow::Result;
use eth_trie_utils::partial_trie::{Nibbles, PartialTrie};
use ethereum_types::{BigEndianHash, H256, U256};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::mpt::extension_to_leaf;
use crate::generation::mpt::{all_mpt_prover_inputs_reversed, AccountRlp};
use crate::generation::TrieInputs;

// TODO: Test with short leaf. Might need to be a storage trie.

#[test]
fn mpt_hash_empty() -> Result<()> {
    let trie_inputs = TrieInputs {
        state_trie: Default::default(),
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };

    test_state_trie(trie_inputs)
}

#[test]
fn mpt_hash_leaf() -> Result<()> {
    let account = AccountRlp {
        nonce: U256::from(1111),
        balance: U256::from(2222),
        storage_root: H256::from_uint(&U256::from(3333)),
        code_hash: H256::from_uint(&U256::from(4444)),
    };
    let account_rlp = rlp::encode(&account);

    let state_trie = PartialTrie::Leaf {
        nibbles: Nibbles {
            count: 3,
            packed: 0xABC.into(),
        },
        value: account_rlp.to_vec(),
    };

    let trie_inputs = TrieInputs {
        state_trie,
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };

    test_state_trie(trie_inputs)
}

#[test]
fn mpt_hash_extension_to_leaf() -> Result<()> {
    let account = AccountRlp {
        nonce: U256::from(1111),
        balance: U256::from(2222),
        storage_root: H256::from_uint(&U256::from(3333)),
        code_hash: H256::from_uint(&U256::from(4444)),
    };
    let account_rlp = rlp::encode(&account);

    let state_trie = extension_to_leaf(account_rlp.to_vec());

    let trie_inputs = TrieInputs {
        state_trie,
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };

    test_state_trie(trie_inputs)
}

#[test]
fn mpt_hash_branch_to_leaf() -> Result<()> {
    let account = AccountRlp {
        nonce: U256::from(1111),
        balance: U256::from(2222),
        storage_root: H256::from_uint(&U256::from(3333)),
        code_hash: H256::from_uint(&U256::from(4444)),
    };
    let account_rlp = rlp::encode(&account);

    let leaf = PartialTrie::Leaf {
        nibbles: Nibbles {
            count: 3,
            packed: 0xABC.into(),
        },
        value: account_rlp.to_vec(),
    };
    let mut children = std::array::from_fn(|_| Box::new(PartialTrie::Empty));
    children[0] = Box::new(leaf);
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

    test_state_trie(trie_inputs)
}

fn test_state_trie(trie_inputs: TrieInputs) -> Result<()> {
    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];
    let mpt_hash_state_trie = KERNEL.global_labels["mpt_hash_state_trie"];

    let initial_stack = vec![0xDEADBEEFu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(load_all_mpts, initial_stack);
    interpreter.generation_state.mpt_prover_inputs = all_mpt_prover_inputs_reversed(&trie_inputs);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    // Now, execute mpt_hash_state_trie.
    interpreter.offset = mpt_hash_state_trie;
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.run()?;

    assert_eq!(
        interpreter.stack().len(),
        1,
        "Expected 1 item on stack, found {:?}",
        interpreter.stack()
    );
    let hash = H256::from_uint(&interpreter.stack()[0]);
    let expected_state_trie_hash = trie_inputs.state_trie.calc_hash();
    assert_eq!(hash, expected_state_trie_hash);

    Ok(())
}
