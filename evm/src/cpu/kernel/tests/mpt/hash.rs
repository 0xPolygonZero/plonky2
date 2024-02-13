use anyhow::{anyhow, Result};
use eth_trie_utils::partial_trie::PartialTrie;
use ethereum_types::{BigEndianHash, H160, H256, U256};
use rand::{thread_rng, Rng};
use smt_utils_hermez::db::MemoryDb;
use smt_utils_hermez::keys::key_balance;
use smt_utils_hermez::smt::{hash_serialize_u256, Smt};
use smt_utils_hermez::utils::hashout2u;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::account_code::initialize_mpts;
use crate::cpu::kernel::tests::mpt::{extension_to_leaf, test_account_1_rlp, test_account_2_rlp};
use crate::generation::TrieInputs;
use crate::Node;

// TODO: Test with short leaf. Might need to be a storage trie.

#[test]
fn smt_hash_empty() -> Result<()> {
    let mut state_smt = Smt::<MemoryDb>::default();
    let trie_inputs = TrieInputs {
        state_smt: state_smt.serialize(),
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
    };

    test_state_trie(trie_inputs)
}

#[test]
fn smt_hash_random() -> Result<()> {
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

        test_state_trie(trie_inputs)?;
    }
    Ok(())
}

// #[test]
// fn mpt_hash_hash() -> Result<()> {
//     let hash = H256::random();
//     let trie_inputs = TrieInputs {
//         state_trie: Node::Hash(hash).into(),
//         transactions_trie: Default::default(),
//         receipts_trie: Default::default(),
//         storage_tries: vec![],
//     };
//
//     test_state_trie(trie_inputs)
// }

fn test_state_trie(trie_inputs: TrieInputs) -> Result<()> {
    let smt_hash_state = KERNEL.global_labels["smt_hash_state"];

    let initial_stack = vec![];
    let mut interpreter = Interpreter::new_with_kernel(0, initial_stack);

    initialize_mpts(&mut interpreter, &trie_inputs);
    assert_eq!(interpreter.stack(), vec![]);

    // Now, execute mpt_hash_state_trie.
    interpreter.generation_state.registers.program_counter = smt_hash_state;
    interpreter
        .push(0xDEADBEEFu32.into())
        .expect("The stack should not overflow");
    interpreter
        .push(2.into()) // Initial length of the trie data segment, unused.
        .expect("The stack should not overflow");
    interpreter.run()?;

    assert_eq!(
        interpreter.stack().len(),
        2,
        "Expected 2 items on stack, found {:?}",
        interpreter.stack()
    );
    let hash = interpreter.stack()[1];
    let expected_state_trie_hash = hash_serialize_u256(&trie_inputs.state_smt);
    assert_eq!(hash, expected_state_trie_hash);

    Ok(())
}
