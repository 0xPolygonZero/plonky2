use anyhow::{anyhow, Result};
use ethereum_types::BigEndianHash;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::account_code::initialize_mpts;
use crate::cpu::kernel::tests::mpt::{extension_to_leaf, test_account_1, test_account_1_rlp};
use crate::generation::TrieInputs;

#[test]
fn mpt_read() -> Result<()> {
    let trie_inputs = TrieInputs {
        state_trie: extension_to_leaf(test_account_1_rlp()),
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };

    let mpt_read = KERNEL.global_labels["mpt_read"];

    let initial_stack = vec![];
    let mut interpreter = Interpreter::new_with_kernel(0, initial_stack);
    initialize_mpts(&mut interpreter, &trie_inputs);
    assert_eq!(interpreter.stack(), vec![]);

    // Now, execute mpt_read on the state trie.
    interpreter.generation_state.registers.program_counter = mpt_read;
    interpreter
        .push(0xdeadbeefu32.into())
        .expect("The stack should not overflow");
    interpreter
        .push(0xABCDEFu64.into())
        .expect("The stack should not overflow");
    interpreter
        .push(6.into())
        .expect("The stack should not overflow");
    interpreter
        .push(interpreter.get_global_metadata_field(GlobalMetadata::StateTrieRoot))
        .expect("The stack should not overflow");
    interpreter.run()?;

    assert_eq!(interpreter.stack().len(), 1);
    let result_ptr = interpreter.stack()[0].as_usize();
    let result = &interpreter.get_trie_data()[result_ptr..][..4];
    assert_eq!(result[0], test_account_1().nonce);
    assert_eq!(result[1], test_account_1().balance);
    // result[2] is the storage root pointer. We won't check that it matches a
    // particular address, since that seems like over-specifying.
    assert_eq!(result[3], test_account_1().code_hash.into_uint());

    Ok(())
}
