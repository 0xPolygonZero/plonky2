use anyhow::{anyhow, Result};
use ethereum_types::{BigEndianHash, H256, U256};
use rand::{thread_rng, Rng};
use smt_utils::account::Account;
use smt_utils::smt::Smt;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::generation::mpt::{all_mpt_prover_inputs_reversed, state_smt_prover_inputs_reversed};
use crate::generation::TrieInputs;

#[test]
fn smt_delete() -> Result<()> {
    let mut rng = thread_rng();
    let n = rng.gen_range(0..100);
    let rand_node = |_| (U256(rng.gen()).into(), Account::rand(10).into());
    let smt = Smt::new((0..n).map(rand_node)).unwrap();

    let new_account = Account::rand(0);
    test_state_smt(smt, U256(rng.gen()), new_account)
}

fn test_state_smt(smt: Smt, k: U256, account: Account) -> Result<()> {
    let trie_inputs = TrieInputs {
        state_smt: smt.serialize(),
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
    };
    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];
    let smt_insert_state = KERNEL.global_labels["smt_insert_state"];
    let smt_delete = KERNEL.global_labels["smt_delete"];
    let smt_hash_state = KERNEL.global_labels["smt_hash_state"];

    let initial_stack = vec![0xDEADBEEFu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(load_all_mpts, initial_stack);
    interpreter.generation_state.state_smt_prover_inputs =
        state_smt_prover_inputs_reversed(&trie_inputs);
    interpreter.generation_state.mpt_prover_inputs =
        all_mpt_prover_inputs_reversed(&trie_inputs).map_err(|_| anyhow!("Invalid MPT data"))?;
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    // Next, execute smt_insert_state.
    interpreter.generation_state.registers.program_counter = smt_insert_state;
    let trie_data = interpreter.get_trie_data_mut();
    let value_ptr = trie_data.len();
    trie_data.push(U256::zero()); // For the key.
    trie_data.push(account.nonce.into());
    trie_data.push(account.balance);
    trie_data.push(U256::zero()); // Empty storage root.
    trie_data.push(account.code_hash.into_uint());
    let trie_data_len = trie_data.len().into();
    interpreter.set_global_metadata_field(GlobalMetadata::TrieDataSize, trie_data_len);
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.push(value_ptr.into()); // value_ptr
    interpreter.push(k); // key
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
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.push(k);
    interpreter.push(state_trie_ptr);
    interpreter.run()?;
    let state_trie_ptr = interpreter.pop();
    interpreter.set_global_metadata_field(GlobalMetadata::StateTrieRoot, state_trie_ptr);

    // Now, execute smt_hash_state.
    interpreter.generation_state.registers.program_counter = smt_hash_state;
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.run()?;

    let state_smt_hash = H256::from_uint(&interpreter.pop());
    let expected_state_smt_hash = smt.root;
    assert_eq!(state_smt_hash, expected_state_smt_hash);

    Ok(())
}
