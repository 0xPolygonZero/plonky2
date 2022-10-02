use anyhow::Result;
use eth_trie_utils::partial_trie::{Nibbles, PartialTrie};
use ethereum_types::{BigEndianHash, H256, U256};
use hex_literal::hex;

use crate::cpu::kernel::aggregator::KERNEL;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::generation::mpt::all_mpt_prover_inputs_reversed;
use crate::generation::TrieInputs;

#[test]
fn mpt_hash() -> Result<()> {
    let nonce = U256::from(1111);
    let balance = U256::from(2222);
    let storage_root = U256::from(3333);
    let code_hash = U256::from(4444);

    let account = &[nonce, balance, storage_root, code_hash];
    let account_rlp = rlp::encode_list(account);

    // TODO: Try this more "advanced" trie.
    // let state_trie = state_trie_ext_to_account_leaf(account_rlp.to_vec());
    let state_trie = PartialTrie::Leaf {
        nibbles: Nibbles {
            count: 3,
            packed: 0xABC.into(),
        },
        value: account_rlp.to_vec(),
    };
    // TODO: It seems like calc_hash isn't giving the expected hash yet, so for now, I'm using a
    // hardcoded hash obtained from py-evm.
    // let state_trie_hash = state_trie.calc_hash();
    let state_trie_hash =
        hex!("e38d6053838fe057c865ec0c74a8f0de21865d74fac222a2d3241fe57c9c3a0f").into();

    let trie_inputs = TrieInputs {
        state_trie,
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };

    let load_all_mpts = KERNEL.global_labels["load_all_mpts"];
    let mpt_hash_state_trie = KERNEL.global_labels["mpt_hash_state_trie"];

    let initial_stack = vec![0xdeadbeefu32.into()];
    let mut interpreter = Interpreter::new_with_kernel(load_all_mpts, initial_stack);
    interpreter.generation_state.mpt_prover_inputs = all_mpt_prover_inputs_reversed(&trie_inputs);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![]);

    // Now, execute mpt_hash_state_trie.
    interpreter.offset = mpt_hash_state_trie;
    interpreter.push(0xDEADBEEFu32.into());
    interpreter.run()?;

    assert_eq!(interpreter.stack().len(), 1);
    let hash = H256::from_uint(&interpreter.stack()[0]);
    assert_eq!(hash, state_trie_hash);

    Ok(())
}
