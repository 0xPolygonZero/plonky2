use anyhow::Result;
use ethereum_types::{BigEndianHash, H256, U256};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::mpt::extension_to_leaf;
use crate::generation::mpt::{all_mpt_prover_inputs_reversed, AccountRlp};
use crate::generation::TrieInputs;

#[test]
fn mpt_read() -> Result<()> {
    let account = AccountRlp {
        nonce: U256::from(1111),
        balance: U256::from(2222),
        storage_root: H256::from_uint(&U256::from(3333)),
        code_hash: H256::from_uint(&U256::from(4444)),
    };
    let account_rlp = rlp::encode(&account);

    let trie_inputs = TrieInputs {
        state_trie: extension_to_leaf(account_rlp.to_vec()),
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };

    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];
    let mpt_read = KERNEL.global_labels["mpt_read"];

    let initial_stack = vec![0xdeadbeefu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(load_all_mpts, initial_stack);
    interpreter.generation_state.mpt_prover_inputs = all_mpt_prover_inputs_reversed(&trie_inputs);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    // Now, execute mpt_read on the state trie.
    interpreter.offset = mpt_read;
    interpreter.push(0xdeadbeefu32.into());
    interpreter.push(0xABCDEFu64.into());
    interpreter.push(6.into());
    interpreter.push(interpreter.get_global_metadata_field(GlobalMetadata::StateTrieRoot));
    interpreter.run()?;

    assert_eq!(interpreter.stack().len(), 1);
    let result_ptr = interpreter.stack()[0].as_usize();
    let result = &interpreter.get_trie_data()[result_ptr..][..4];
    assert_eq!(result[0], account.nonce);
    assert_eq!(result[1], account.balance);
    assert_eq!(result[2], account.storage_root.into_uint());
    assert_eq!(result[3], account.code_hash.into_uint());

    Ok(())
}
