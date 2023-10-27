use anyhow::{anyhow, Result};
use ethereum_types::{Address, BigEndianHash, H256, U256};
use keccak_hash::keccak;
use rand::{thread_rng, Rng};
use smt_utils::account::Account;
use smt_utils::smt::Smt;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::generation::mpt::all_mpt_prover_inputs_reversed;

// Test account with a given code hash.
fn test_account(balance: U256) -> Account {
    Account {
        nonce: 1111,
        balance,
        storage_smt: Smt::empty(),
        code_hash: H256::from_uint(&U256::from(8888)),
    }
}

// Stolen from `tests/mpt/insert.rs`
// Prepare the interpreter by inserting the account in the state trie.
fn prepare_interpreter(
    interpreter: &mut Interpreter,
    address: Address,
    account: Account,
) -> Result<()> {
    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];
    let smt_insert_state = KERNEL.global_labels["smt_insert_state"];
    let smt_hash_state = KERNEL.global_labels["smt_hash_state"];
    let mut state_smt = Smt::empty();
    let trie_inputs = Default::default();

    interpreter.generation_state.registers.program_counter = load_all_mpts;
    interpreter.push(0xDEADBEEFu32.into());

    interpreter.generation_state.mpt_prover_inputs =
        all_mpt_prover_inputs_reversed(&trie_inputs)
            .map_err(|err| anyhow!("Invalid MPT data: {:?}", err))?;
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    let k = keccak(address.to_fixed_bytes());
    // Next, execute smt_insert_state.
    interpreter.generation_state.registers.program_counter = smt_insert_state;
    let trie_data = interpreter.get_trie_data_mut();
    if trie_data.is_empty() {
        // In the assembly we skip over 0, knowing trie_data[0:4] = 0 by default.
        // Since we don't explicitly set it to 0, we need to do so here.
        trie_data.extend((0..4).map(|_| U256::zero()));
    }
    let value_ptr = trie_data.len();
    trie_data.push(U256::zero()); // For key.
    trie_data.push(account.nonce.into());
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
    interpreter.push(k.into_uint());

    interpreter.run()?;
    assert_eq!(
        interpreter.stack().len(),
        0,
        "Expected empty stack after insert, found {:?}",
        interpreter.stack()
    );

    // Now, execute mpt_hash_state_trie.
    interpreter.generation_state.registers.program_counter = smt_hash_state;
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.run()?;

    assert_eq!(
        interpreter.stack().len(),
        1,
        "Expected 1 item on stack after hashing, found {:?}",
        interpreter.stack()
    );
    let hash = H256::from_uint(&interpreter.stack()[0]);

    state_smt.insert(k.into(), account.into()).unwrap();
    let expected_state_trie_hash = state_smt.root;
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
    prepare_interpreter(&mut interpreter, address, account)?;

    // Test `balance`
    interpreter.generation_state.registers.program_counter = KERNEL.global_labels["balance"];
    interpreter.pop();
    assert!(interpreter.stack().is_empty());
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.push(U256::from_big_endian(address.as_bytes()));
    interpreter.run()?;

    assert_eq!(interpreter.stack(), vec![balance]);

    Ok(())
}
