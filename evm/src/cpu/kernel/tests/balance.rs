use anyhow::Result;
use eth_trie_utils::partial_trie::PartialTrie;
use ethereum_types::{Address, BigEndianHash, H256, U256};
use keccak_hash::keccak;
use rand::{thread_rng, Rng};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::mpt::nibbles_64;
use crate::generation::mpt::{all_mpt_prover_inputs_reversed, AccountRlp};

// Test account with a given code hash.
fn test_account(balance: U256) -> AccountRlp {
    AccountRlp {
        nonce: U256::from(1111),
        balance,
        storage_root: PartialTrie::Empty.calc_hash(),
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
    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];
    let mpt_insert_state_trie = KERNEL.global_labels["mpt_insert_state_trie"];
    let mpt_hash_state_trie = KERNEL.global_labels["mpt_hash_state_trie"];
    let mut state_trie: PartialTrie = Default::default();
    let trie_inputs = Default::default();

    interpreter.offset = load_all_mpts;
    interpreter.push(0xDEADBEEFu32.into());

    interpreter.generation_state.mpt_prover_inputs = all_mpt_prover_inputs_reversed(&trie_inputs);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    let k = nibbles_64(U256::from_big_endian(
        keccak(address.to_fixed_bytes()).as_bytes(),
    ));
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

    state_trie.insert(k, rlp::encode(account).to_vec());
    let expected_state_trie_hash = state_trie.calc_hash();
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
    interpreter.offset = KERNEL.global_labels["balance"];
    interpreter.pop();
    assert!(interpreter.stack().is_empty());
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.push(U256::from_big_endian(address.as_bytes()));
    interpreter.run()?;

    assert_eq!(interpreter.stack(), vec![balance]);

    Ok(())
}
