use anyhow::{anyhow, Result};
use eth_trie_utils::partial_trie::{HashedPartialTrie, PartialTrie};
use ethereum_types::{Address, BigEndianHash, H256, U256};
use keccak_hash::keccak;
use rand::{thread_rng, Rng};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::account_code::initialize_mpts;
use crate::cpu::kernel::tests::mpt::nibbles_64;
use crate::generation::mpt::AccountRlp;
use crate::Node;

// Test account with a given code hash.
fn test_account(balance: U256) -> AccountRlp {
    AccountRlp {
        nonce: U256::from(1111),
        balance,
        storage_root: HashedPartialTrie::from(Node::Empty).hash(),
        code_hash: H256::from_uint(&U256::from(8888)),
    }
}

// Stolen from `tests/mpt/insert.rs`
// Prepare the interpreter by inserting the account in the state trie.
fn prepare_interpreter(
    interpreter: &mut Interpreter,
    address: Address,
    account: &AccountRlp,
) -> Result<()> {
    let mpt_insert_state_trie = KERNEL.global_labels["mpt_insert_state_trie"];
    let mpt_hash_state_trie = KERNEL.global_labels["mpt_hash_state_trie"];
    let mut state_trie: HashedPartialTrie = Default::default();
    let trie_inputs = Default::default();

    initialize_mpts(interpreter, &trie_inputs);
    assert_eq!(interpreter.stack(), vec![]);

    let k = nibbles_64(U256::from_big_endian(
        keccak(address.to_fixed_bytes()).as_bytes(),
    ));
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
    interpreter
        .push(0xDEADBEEFu32.into())
        .expect("The stack should not overflow");
    interpreter
        .push(value_ptr.into())
        .expect("The stack should not overflow"); // value_ptr
    interpreter
        .push(k.try_into_u256().unwrap())
        .expect("The stack should not overflow"); // key

    interpreter.run()?;
    assert_eq!(
        interpreter.stack().len(),
        0,
        "Expected empty stack after insert, found {:?}",
        interpreter.stack()
    );

    // Now, execute mpt_hash_state_trie.
    interpreter.generation_state.registers.program_counter = mpt_hash_state_trie;
    interpreter
        .push(0xDEADBEEFu32.into())
        .expect("The stack should not overflow");
    interpreter
        .push(1.into()) // Initial trie data segment size, unused.
        .expect("The stack should not overflow");
    interpreter.run()?;

    assert_eq!(
        interpreter.stack().len(),
        2,
        "Expected 2 items on stack after hashing, found {:?}",
        interpreter.stack()
    );
    let hash = H256::from_uint(&interpreter.stack()[1]);

    state_trie.insert(k, rlp::encode(account).to_vec());
    let expected_state_trie_hash = state_trie.hash();
    assert_eq!(hash, expected_state_trie_hash);

    Ok(())
}

#[test]
fn test_balance() -> Result<()> {
    let mut rng = thread_rng();
    let balance = U256(rng.gen());
    let account = test_account(balance);

    let mut interpreter = Interpreter::new_with_kernel(0, vec![]);
    let address: Address = rng.gen();
    // Prepare the interpreter by inserting the account in the state trie.
    prepare_interpreter(&mut interpreter, address, &account)?;

    // Test `balance`
    interpreter.generation_state.registers.program_counter = KERNEL.global_labels["balance"];
    interpreter.pop().expect("The stack should not be empty");
    interpreter.pop().expect("The stack should not be empty");
    assert!(interpreter.stack().is_empty());
    interpreter
        .push(0xDEADBEEFu32.into())
        .expect("The stack should not overflow");
    interpreter
        .push(U256::from_big_endian(address.as_bytes()))
        .expect("The stack should not overflow");
    interpreter.run()?;

    assert_eq!(interpreter.stack(), vec![balance]);

    Ok(())
}
