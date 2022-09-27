use anyhow::Result;
use eth_trie_utils::partial_trie::{Nibbles, PartialTrie};
use ethereum_types::U256;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::global_metadata::GlobalMetadata;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::generation::mpt::all_mpt_prover_inputs_reversed;
use crate::generation::TrieInputs;

#[test]
fn mpt_read() -> Result<()> {
    let nonce = U256::from(1111);
    let balance = U256::from(2222);
    let storage_root = U256::from(3333);
    let code_hash = U256::from(4444);

    let account = &[nonce, balance, storage_root, code_hash];
    let account_rlp = rlp::encode_list(account);

    let trie_inputs = TrieInputs {
        state_trie: PartialTrie::Leaf {
            nibbles: Nibbles {
                count: 2,
                packed: 123.into(),
            },
            value: account_rlp.to_vec(),
        },
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
    interpreter.push(2.into());
    interpreter.push(123.into());
    interpreter.push(interpreter.get_global_metadata_field(GlobalMetadata::StateTrieRoot));
    interpreter.run()?;

    assert_eq!(interpreter.stack().len(), 1);
    let result_ptr = interpreter.stack()[0].as_usize();
    let result = &interpreter.get_trie_data()[result_ptr..result_ptr + 4];
    assert_eq!(result, account);

    Ok(())
}
