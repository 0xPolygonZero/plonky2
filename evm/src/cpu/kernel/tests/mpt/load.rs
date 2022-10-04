use anyhow::Result;
use ethereum_types::U256;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::constants::trie_type::PartialTrieType;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::mpt::state_trie_ext_to_account_leaf;
use crate::generation::mpt::all_mpt_prover_inputs_reversed;
use crate::generation::TrieInputs;

#[test]
fn load_all_mpts() -> Result<()> {
    let nonce = U256::from(1111);
    let balance = U256::from(2222);
    let storage_root = U256::from(3333);
    let code_hash = U256::from(4444);

    let account_rlp = rlp::encode_list(&[nonce, balance, storage_root, code_hash]);

    let trie_inputs = TrieInputs {
        state_trie: state_trie_ext_to_account_leaf(account_rlp.to_vec()),
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };

    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];

    let initial_stack = vec![0xdeadbeefu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(load_all_mpts, initial_stack);
    interpreter.generation_state.mpt_prover_inputs = all_mpt_prover_inputs_reversed(&trie_inputs);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    let type_empty = U256::from(PartialTrieType::Empty as u32);
    let type_extension = U256::from(PartialTrieType::Extension as u32);
    let type_leaf = U256::from(PartialTrieType::Leaf as u32);
    assert_eq!(
        interpreter.get_trie_data(),
        vec![
            0.into(), // First address is unused, so that 0 can be treated as a null pointer.
            type_extension,
            3.into(),     // 3 nibbles
            0xABC.into(), // key part
            5.into(),     // Pointer to the leaf node immediately below.
            type_leaf,
            3.into(),     // 3 nibbles
            0xDEF.into(), // key part
            nonce,
            balance,
            storage_root,
            code_hash,
            type_empty,
            type_empty,
        ]
    );

    assert_eq!(
        interpreter.get_global_metadata_field(GlobalMetadata::NumStorageTries),
        trie_inputs.storage_tries.len().into()
    );

    Ok(())
}
