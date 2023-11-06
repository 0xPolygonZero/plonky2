use anyhow::{anyhow, Result};
use ethereum_types::{BigEndianHash, H256, U256};
use rand::{thread_rng, Rng};
use smt_utils::account::Account;
use smt_utils::smt::Smt;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::generation::mpt::{all_mpt_prover_inputs_reversed, state_smt_prover_inputs_reversed};
use crate::generation::TrieInputs;

// TODO: Test with short leaf. Might need to be a storage trie.

#[test]
fn smt_hash_empty() -> Result<()> {
    let smt = Smt::empty();
    test_state_smt(smt)
}

#[test]
fn smt_hash() -> Result<()> {
    let n = 100;
    let mut rng = thread_rng();
    let rand_node = |_| (U256(rng.gen()).into(), Account::rand(10).into());
    let smt = Smt::new((0..n).map(rand_node)).unwrap();

    test_state_smt(smt)
}

fn test_state_smt(state_smt: Smt) -> Result<()> {
    let trie_inputs = TrieInputs {
        state_smt: state_smt.serialize(),
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
    };
    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];
    let smt_hash_state = KERNEL.global_labels["smt_hash_state"];

    let initial_stack = vec![0xDEADBEEFu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(load_all_mpts, initial_stack);
    interpreter.generation_state.mpt_prover_inputs =
        all_mpt_prover_inputs_reversed(&trie_inputs).map_err(|_| anyhow!("Invalid MPT data"))?;
    interpreter.generation_state.state_smt_prover_inputs =
        state_smt_prover_inputs_reversed(&trie_inputs);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    // Now, execute smt_hash_state.
    interpreter.generation_state.registers.program_counter = smt_hash_state;
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.run()?;

    assert_eq!(
        interpreter.stack().len(),
        1,
        "Expected 1 item on stack, found {:?}",
        interpreter.stack()
    );
    let hash = H256::from_uint(&interpreter.stack()[0]);
    let expected_state_trie_hash = state_smt.root;
    assert_eq!(hash, expected_state_trie_hash);

    Ok(())
}
