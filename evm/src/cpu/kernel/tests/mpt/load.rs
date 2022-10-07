use anyhow::Result;
use ethereum_types::{BigEndianHash, H256, U256};

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::constants::global_metadata::GlobalMetadata;
use crate::cpu::kernel::constants::trie_type::PartialTrieType;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::mpt::extension_to_leaf;
use crate::generation::mpt::{all_mpt_prover_inputs_reversed, AccountRlp};
use crate::generation::TrieInputs;

#[test]
fn load_all_mpts() -> Result<()> {
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
            4.into(),     // value length
            account.nonce,
            account.balance,
            account.storage_root.into_uint(),
            account.code_hash.into_uint(),
            type_empty, // txn trie
            type_empty, // receipt trie
        ]
    );

    assert_eq!(
        interpreter.get_global_metadata_field(GlobalMetadata::NumStorageTries),
        trie_inputs.storage_tries.len().into()
    );

    Ok(())
}
