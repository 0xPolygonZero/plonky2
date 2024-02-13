use anyhow::{anyhow, Result};
use eth_trie_utils::nibbles::Nibbles;
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{BigEndianHash, H160, H256, U256, U512};
use rand::{random, thread_rng, Rng};
use smt_utils_hermez::db::MemoryDb;
use smt_utils_hermez::keys::key_balance;
use smt_utils_hermez::smt::{Key, Smt};
use smt_utils_hermez::utils::{hashout2u, key2u};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::account_code::initialize_mpts;
use crate::cpu::kernel::tests::mpt::{nibbles_64, test_account_1_rlp, test_account_2};
use crate::generation::mpt::AccountRlp;
use crate::generation::TrieInputs;
use crate::Node;

#[test]
fn smt_delete_empty() -> Result<()> {
    test_state_trie(
        Smt::<MemoryDb>::default(),
        key_balance(H160(random())),
        U256(random()),
    )
}

#[test]
fn smt_delete_random() -> Result<()> {
    const N: usize = 100;
    let mut rng = thread_rng();
    for _iter in 0..N {
        let mut state_smt = Smt::<MemoryDb>::default();
        let num_keys: usize = rng.gen_range(0..100);
        for _ in 0..num_keys {
            let key = key_balance(H160(rng.gen()));
            let value = U256(rng.gen());
            state_smt.set(key, value);
        }
        let trie_inputs = TrieInputs {
            state_smt: state_smt.serialize(),
            transactions_trie: Default::default(),
            receipts_trie: Default::default(),
        };

        let key = key_balance(H160(rng.gen()));
        let value = U256(rng.gen());
        test_state_trie(state_smt, key, value)?;
    }
    Ok(())
}

/// Note: The account's storage_root is ignored, as we can't insert a new storage_root without the
/// accompanying trie data. An empty trie's storage_root is used instead.
fn test_state_trie(state_smt: Smt<MemoryDb>, k: Key, value: U256) -> Result<()> {
    let trie_inputs = TrieInputs {
        state_smt: state_smt.serialize(),
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
    };
    let smt_insert_state = KERNEL.global_labels["smt_insert_state"];
    let smt_delete = KERNEL.global_labels["smt_delete"];
    let smt_hash = KERNEL.global_labels["smt_hash"];

    let initial_stack = vec![];
    let mut interpreter = Interpreter::new_with_kernel(0, initial_stack);

    initialize_mpts(&mut interpreter, &trie_inputs);
    assert_eq!(interpreter.stack(), vec![]);

    // Next, execute smt_insert_state.
    interpreter.generation_state.registers.program_counter = smt_insert_state;
    let trie_data = interpreter.get_trie_data_mut();
    if trie_data.is_empty() {
        // In the assembly we skip over 0, knowing trie_data[0] = 0 by default.
        // Since we don't explicitly set it to 0, we need to do so here.
        trie_data.push(0.into());
        trie_data.push(0.into());
    }
    let len = trie_data.len();
    interpreter.set_global_metadata_field(GlobalMetadata::TrieDataSize, len.into());
    interpreter
        .push(0xDEADBEEFu32.into())
        .expect("The stack should not overflow");
    interpreter
        .push(value)
        .expect("The stack should not overflow");
    interpreter
        .push(key2u(k))
        .expect("The stack should not overflow"); // key
    interpreter.run()?;
    assert_eq!(
        interpreter.stack().len(),
        0,
        "Expected empty stack after insert, found {:?}",
        interpreter.stack()
    );

    // Next, execute smt_delete, deleting the account we just inserted.
    let state_trie_ptr = interpreter.get_global_metadata_field(GlobalMetadata::StateTrieRoot);
    interpreter.generation_state.registers.program_counter = smt_delete;
    interpreter
        .push(0xDEADBEEFu32.into())
        .expect("The stack should not overflow");
    interpreter
        .push(key2u(k))
        .expect("The stack should not overflow");
    interpreter
        .push(state_trie_ptr)
        .expect("The stack should not overflow");
    interpreter.run()?;
    let state_trie_ptr = interpreter.pop().expect("The stack should not be empty");

    // Now, execute smt_hash_state.
    interpreter.generation_state.registers.program_counter = smt_hash;
    interpreter
        .push(0xDEADBEEFu32.into())
        .expect("The stack should not overflow");
    interpreter
        .push(2.into()) // Initial length of the trie data segment, unused.
        .expect("The stack should not overflow");
    interpreter
        .push(state_trie_ptr)
        .expect("The stack should not overflow");
    interpreter.run()?;

    let state_smt_hash = interpreter.pop().expect("The stack should not be empty");
    let expected_state_smt_hash = hashout2u(state_smt.root);
    assert_eq!(state_smt_hash, expected_state_smt_hash);

    Ok(())
}
