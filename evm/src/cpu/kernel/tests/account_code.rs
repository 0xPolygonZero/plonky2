use std::collections::HashMap;

use anyhow::{anyhow, Result};
use ethereum_types::{Address, BigEndianHash, H256, U256};
use keccak_hash::keccak;
use rand::{random, thread_rng, Rng};
use smt_utils::account::Account;
use smt_utils::smt::Smt;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::context_metadata::ContextMetadata::GasLimit;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::generation::mpt::{all_mpt_prover_inputs_reversed, state_smt_prover_inputs_reversed};
use crate::memory::segments::Segment;

// Test account with a given code hash.
fn test_account(code: &[u8]) -> Account {
    Account {
        nonce: 1111,
        balance: U256::from(2222),
        storage_smt: Smt::empty(),
        code_hash: keccak(code),
    }
}

fn random_code() -> Vec<u8> {
    let mut rng = thread_rng();
    let num_bytes = rng.gen_range(0..1000);
    (0..num_bytes).map(|_| rng.gen()).collect()
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

    interpreter.generation_state.state_smt_prover_inputs =
        state_smt_prover_inputs_reversed(&trie_inputs);
    interpreter.generation_state.mpt_prover_inputs =
        all_mpt_prover_inputs_reversed(&trie_inputs)
            .map_err(|err| anyhow!("Invalid MPT data: {:?}", err))?;
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    let k = keccak(address.to_fixed_bytes());
    // Next, execute smt_insert_state.
    interpreter.generation_state.registers.program_counter = smt_insert_state;
    let trie_data = interpreter.get_trie_data_mut();
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
    interpreter.push(k.into_uint()); // key

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
fn test_extcodesize() -> Result<()> {
    let code = random_code();
    let account = test_account(&code);

    let mut interpreter = Interpreter::new_with_kernel(0, vec![]);
    let address: Address = random();
    // Prepare the interpreter by inserting the account in the state trie.
    prepare_interpreter(&mut interpreter, address, account)?;

    let extcodesize = KERNEL.global_labels["extcodesize"];

    // Test `extcodesize`
    interpreter.generation_state.registers.program_counter = extcodesize;
    interpreter.pop();
    assert!(interpreter.stack().is_empty());
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.push(U256::from_big_endian(address.as_bytes()));
    interpreter.generation_state.inputs.contract_code =
        HashMap::from([(keccak(&code), code.clone())]);
    interpreter.run()?;

    assert_eq!(interpreter.stack(), vec![code.len().into()]);

    Ok(())
}

#[test]
fn test_extcodecopy() -> Result<()> {
    let code = random_code();
    let account = test_account(&code);

    let mut interpreter = Interpreter::new_with_kernel(0, vec![]);
    let address: Address = thread_rng().gen();
    // Prepare the interpreter by inserting the account in the state trie.
    prepare_interpreter(&mut interpreter, address, account)?;

    interpreter.generation_state.memory.contexts[interpreter.context].segments
        [Segment::ContextMetadata as usize]
        .set(GasLimit as usize, U256::from(1000000000000u64) << 192);

    let extcodecopy = KERNEL.global_labels["sys_extcodecopy"];

    // Put random data in main memory and the `KernelAccountCode` segment for realism.
    let mut rng = thread_rng();
    for i in 0..2000 {
        interpreter.generation_state.memory.contexts[interpreter.context].segments
            [Segment::MainMemory as usize]
            .set(i, U256::from(rng.gen::<u8>()));
        interpreter.generation_state.memory.contexts[interpreter.context].segments
            [Segment::KernelAccountCode as usize]
            .set(i, U256::from(rng.gen::<u8>()));
    }

    // Random inputs
    let dest_offset = rng.gen_range(0..3000);
    let offset = rng.gen_range(0..1500);
    let size = rng.gen_range(0..1500);

    // Test `extcodecopy`
    interpreter.generation_state.registers.program_counter = extcodecopy;
    interpreter.pop();
    assert!(interpreter.stack().is_empty());
    interpreter.push(size.into());
    interpreter.push(offset.into());
    interpreter.push(dest_offset.into());
    interpreter.push(U256::from_big_endian(address.as_bytes()));
    interpreter.push(0xDEADBEEFu32.into()); // kexit_info
    interpreter.generation_state.inputs.contract_code =
        HashMap::from([(keccak(&code), code.clone())]);
    interpreter.run()?;

    assert!(interpreter.stack().is_empty());
    // Check that the code was correctly copied to memory.
    for i in 0..size {
        let memory = interpreter.generation_state.memory.contexts[interpreter.context].segments
            [Segment::MainMemory as usize]
            .get(dest_offset + i);
        assert_eq!(
            memory,
            code.get(offset + i).copied().unwrap_or_default().into()
        );
    }

    Ok(())
}
