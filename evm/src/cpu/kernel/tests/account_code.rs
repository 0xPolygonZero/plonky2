use std::collections::HashMap;
use std::str::FromStr;

use anyhow::Result;
use eth_trie_utils::partial_trie::PartialTrie;
use ethereum_types::{BigEndianHash, H256, U256};

use crate::cpu::kernel::aggregator::{combined_kernel, KERNEL};
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::mpt::{extension_to_leaf, nibbles_64};
use crate::generation::mpt::{all_mpt_prover_inputs_reversed, AccountRlp};
use crate::generation::TrieInputs;

fn test_account_1() -> AccountRlp {
    AccountRlp {
        nonce: U256::from(1111),
        balance: U256::from(2222),
        storage_root: PartialTrie::Empty.calc_hash(),
        code_hash: H256::from_str(
            "2636a8beb2c41b8ccafa9a55a5a5e333892a83b491df3a67d2768946a9f9c6dc",
        )
        .unwrap(),
    }
}

pub(crate) fn test_account_1_rlp() -> Vec<u8> {
    rlp::encode(&test_account_1()).to_vec()
}

#[test]
fn test_extcodecopy() -> Result<()> {
    let state_trie: PartialTrie = Default::default();
    let trie_inputs = Default::default();
    let account = test_account_1();

    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];
    let mpt_insert_state_trie = KERNEL.global_labels["mpt_insert_state_trie"];
    let mpt_hash_state_trie = KERNEL.global_labels["mpt_hash_state_trie"];
    let extcodecopy = KERNEL.global_labels["extcodecopy"];
    let extcodesize = KERNEL.global_labels["extcodesize"];

    let initial_stack = vec![0xDEADBEEFu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(load_all_mpts, initial_stack);
    interpreter.generation_state.mpt_prover_inputs = all_mpt_prover_inputs_reversed(&trie_inputs);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    let k = nibbles_64(U256::from_str(
        "5380c7b7ae81a58eb98d9c78de4a1fd7fd9535fc953ed2be602daaa41767312a",
    )?);
    // Next, execute mpt_insert_state_trie.
    interpreter.offset = mpt_insert_state_trie;
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
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.push(value_ptr.into()); // value_ptr
    interpreter.push(k.packed); // key

    dbg!(interpreter.stack());
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

    let updated_trie = state_trie.insert(k, rlp::encode(&account).to_vec());
    let expected_state_trie_hash = updated_trie.calc_hash();
    assert_eq!(hash, expected_state_trie_hash);

    // let initial_stack = vec![0.into()];
    interpreter.pop();
    interpreter.push(U256::zero());
    dbg!(interpreter.stack());
    interpreter.offset = extcodesize;
    // let mut interpreter = Interpreter::new_with_kernel(extcodesize, initial_stack);
    // interpreter.generation_state.mpt_prover_inputs = all_mpt_prover_inputs_reversed(&trie_inputs);
    interpreter.generation_state.inputs.contract_code = HashMap::from([(
        H256::from_str("2636a8beb2c41b8ccafa9a55a5a5e333892a83b491df3a67d2768946a9f9c6dc")?,
        vec![0x13, 0x37],
    )]);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![2.into()]);

    Ok(())
}
