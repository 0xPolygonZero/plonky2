use std::collections::HashMap;
use std::str::FromStr;

use anyhow::Result;
use ethereum_types::{BigEndianHash, H256, U256};

use crate::cpu::kernel::aggregator::combined_kernel;
use crate::cpu::kernel::interpreter::Interpreter;
use crate::cpu::kernel::tests::mpt::extension_to_leaf;
use crate::generation::mpt::{all_mpt_prover_inputs_reversed, AccountRlp};
use crate::generation::TrieInputs;

fn test_account_1() -> AccountRlp {
    AccountRlp {
        nonce: U256::from(1111),
        balance: U256::from(2222),
        storage_root: H256::from_uint(&U256::from(3333)),
        code_hash: H256::from_uint(&U256::from(4444)),
    }
}

pub(crate) fn test_account_1_rlp() -> Vec<u8> {
    rlp::encode(&test_account_1()).to_vec()
}

#[test]
fn test_extcodecopy() -> Result<()> {
    let trie_inputs = TrieInputs {
        state_trie: extension_to_leaf(test_account_1_rlp()),
        transactions_trie: Default::default(),
        receipts_trie: Default::default(),
        storage_tries: vec![],
    };
    let kernel = combined_kernel();
    let extcodecopy = kernel.global_labels["extcodecopy"];
    let extcodesize = kernel.global_labels["extcodesize"];

    let initial_stack = vec![0.into()];
    let mut interpreter = Interpreter::new_with_kernel(extcodesize, initial_stack);
    interpreter.generation_state.mpt_prover_inputs = all_mpt_prover_inputs_reversed(&trie_inputs);
    interpreter.generation_state.inputs.contract_code = HashMap::from([(
        H256::from_str("2636a8beb2c41b8ccafa9a55a5a5e333892a83b491df3a67d2768946a9f9c6dc")?,
        vec![0x13, 0x37],
    )]);
    interpreter.run()?;
    assert_eq!(interpreter.stack(), vec![2.into()]);

    Ok(())
}
