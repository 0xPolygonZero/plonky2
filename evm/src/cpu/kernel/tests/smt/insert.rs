use anyhow::{anyhow, Result};
use eth_trie_utils::partial_trie::PartialTrie;
use ethereum_types::{BigEndianHash, H256, U256};
use rand::{thread_rng, Rng};
use smt_utils::account::Account;
use smt_utils::smt::{AccountOrValue, Smt, ValOrHash};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::mpt::{extension_to_leaf, test_account_1_rlp, test_account_2_rlp};
use crate::generation::mpt::{
    all_mpt_prover_inputs_reversed, state_smt_prover_inputs, state_smt_prover_inputs_reversed,
};
use crate::generation::TrieInputs;
use crate::memory::segments::Segment;
use crate::Node;

#[test]
fn smt_insert_state() -> Result<()> {
    let n = 100;
    let mut rng = thread_rng();
    let rand_node = |_| {
        (
            U256(rng.gen()).into(),
            ValOrHash::Val(AccountOrValue::Account(Account::rand(10))),
        )
    };
    let smt = Smt::new((0..n).map(rand_node)).unwrap();
    let new_key = U256(rng.gen());
    let new_account = Account::rand(0);

    test_state_smt(smt, new_key, new_account)
}

fn test_state_smt(mut state_smt: Smt, new_key: U256, new_account: Account) -> Result<()> {
    let trie_inputs = TrieInputs {
        state_trie: state_smt.serialize(),
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };
    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];

    let initial_stack = vec![0xDEADBEEFu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(load_all_mpts, initial_stack);
    interpreter.generation_state.mpt_prover_inputs =
        all_mpt_prover_inputs_reversed(&trie_inputs).map_err(|_| anyhow!("Invalid MPT data"))?;
    interpreter.generation_state.state_smt_prover_inputs =
        state_smt_prover_inputs_reversed(&trie_inputs);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    let state_root = interpreter.get_global_metadata_field(GlobalMetadata::StateTrieRoot);
    let trie_data_size = interpreter.get_global_metadata_field(GlobalMetadata::TrieDataSize);
    let trie_data = interpreter.get_trie_data_mut();
    trie_data.push(U256::zero()); // For key
    let mut packed_account = new_account.pack_u256();
    packed_account[2] = U256::zero(); // No storage SMT.
    for (i, x) in (trie_data_size.as_usize() + 1..).zip(packed_account) {
        if i < trie_data.len() {
            trie_data[i] = x;
        } else {
            trie_data.push(x);
        }
    }
    let len = trie_data.len();
    interpreter.set_global_metadata_field(GlobalMetadata::TrieDataSize, len.into());

    let smt_insert = KERNEL.global_labels["smt_insert"];
    // Now, execute smt_insert.
    interpreter.generation_state.registers.program_counter = smt_insert;
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.push(trie_data_size);
    interpreter.push(new_key);
    interpreter.push(state_root);
    interpreter.run()?;

    assert_eq!(
        interpreter.stack().len(),
        1,
        "Expected 1 item on stack, found {:?}",
        interpreter.stack()
    );
    let smt_hash = KERNEL.global_labels["smt_hash"];
    interpreter.generation_state.registers.program_counter = smt_hash;
    let ptr = interpreter.stack()[0];
    interpreter.pop();
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.push(ptr);
    interpreter.run()?;
    let hash = interpreter.pop();

    state_smt
        .insert(
            new_key.into(),
            ValOrHash::Val(AccountOrValue::Account(new_account)),
        )
        .unwrap();
    let expected_hash = state_smt.root;

    assert_eq!(hash, expected_hash.into_uint());

    Ok(())
}

#[test]
fn smt_insert_storage() -> Result<()> {
    let n = 100;
    let mut rng = thread_rng();
    let rand_node = |_| {
        (
            U256(rng.gen()).into(),
            ValOrHash::Val(AccountOrValue::Value(U256(rng.gen()))),
        )
    };
    let smt = Smt::new((0..n).map(rand_node)).unwrap();
    let new_key = U256(rng.gen());
    let new_val = U256(rng.gen());

    test_storage_smt(smt, new_key, new_val)
}

fn test_storage_smt(mut storage_smt: Smt, new_key: U256, new_val: U256) -> Result<()> {
    let initial_stack = vec![0xDEADBEEFu32.into()];
    let smt_insert = KERNEL.global_labels["smt_insert"];
    let mut interpreter = Interpreter::new_with_kernel(smt_insert, initial_stack);
    let trie_data = storage_smt.serialize();
    let len = trie_data.len();
    interpreter.generation_state.memory.contexts[0].segments[Segment::TrieData as usize].content =
        trie_data;
    interpreter.set_global_metadata_field(GlobalMetadata::TrieDataSize, len.into());
    interpreter.set_global_metadata_field(GlobalMetadata::StateTrieRoot, 2.into());

    let state_root = interpreter.get_global_metadata_field(GlobalMetadata::StateTrieRoot);
    let trie_data_size = interpreter.get_global_metadata_field(GlobalMetadata::TrieDataSize);
    let trie_data = &mut interpreter.generation_state.memory.contexts[0].segments
        [Segment::TrieData as usize]
        .content;
    trie_data.push(U256::zero()); // For key
    trie_data.push(new_val);
    let len = trie_data.len();
    interpreter.set_global_metadata_field(GlobalMetadata::TrieDataSize, len.into());

    let smt_insert = KERNEL.global_labels["smt_insert"];
    // Now, execute smt_insert.
    interpreter.generation_state.registers.program_counter = smt_insert;
    interpreter.push(trie_data_size);
    interpreter.push(new_key);
    interpreter.push(state_root);
    interpreter.run()?;

    assert_eq!(
        interpreter.stack().len(),
        1,
        "Expected 1 item on stack, found {:?}",
        interpreter.stack()
    );

    let smt_hash = KERNEL.global_labels["smt_hash"];
    interpreter.generation_state.registers.program_counter = smt_hash;
    interpreter.generation_state.memory.contexts[0].segments[Segment::KernelGeneral as usize]
        .content
        .resize(13371338, U256::zero());
    interpreter.generation_state.memory.contexts[0].segments[Segment::KernelGeneral as usize]
        .content[13371337] = U256::one(); // To hash storage trie.
    let ptr = interpreter.stack()[0];
    interpreter.pop();
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.push(ptr);
    interpreter.run()?;
    let hash = interpreter.pop();

    storage_smt
        .insert(
            new_key.into(),
            ValOrHash::Val(AccountOrValue::Value(new_val)),
        )
        .unwrap();
    let expected_hash = storage_smt.root;

    assert_eq!(hash, expected_hash.into_uint());

    Ok(())
}
