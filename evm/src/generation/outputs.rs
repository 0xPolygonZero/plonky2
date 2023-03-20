use std::collections::HashMap;

use ethereum_types::{Address, BigEndianHash, H256, U256};
use plonky2::field::types::Field;

use crate::cpu::kernel::constants::global_metadata::GlobalMetadata::StateTrieRoot;
use crate::generation::state::GenerationState;
use crate::generation::trie_extractor::{
    read_state_trie_value, read_storage_trie_value, read_trie, AccountTrieRecord,
};

/// The post-state after trace generation; intended for debugging.
#[derive(Clone, Debug)]
pub struct GenerationOutputs {
    pub accounts: HashMap<AddressOrStateKey, AccountOutput>,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum AddressOrStateKey {
    Address(Address),
    StateKey(H256),
}

#[derive(Clone, Debug)]
pub struct AccountOutput {
    pub balance: U256,
    pub nonce: u64,
    pub code: Vec<u8>,
    pub storage: HashMap<U256, U256>,
}

pub(crate) fn get_outputs<F: Field>(state: &mut GenerationState<F>) -> GenerationOutputs {
    // First observe all addresses passed in the by caller.
    for address in state.inputs.addresses.clone() {
        state.observe_address(address);
    }

    let account_map = read_trie::<AccountTrieRecord>(
        &state.memory,
        state.memory.read_global_metadata(StateTrieRoot).as_usize(),
        read_state_trie_value,
    );

    let accounts = account_map
        .into_iter()
        .map(|(state_key_nibbles, account)| {
            assert_eq!(
                state_key_nibbles.count, 64,
                "Each state key should have 64 nibbles = 256 bits"
            );
            let state_key_h256 = H256::from_uint(&state_key_nibbles.packed);

            let addr_or_state_key =
                if let Some(address) = state.state_key_to_address.get(&state_key_h256) {
                    AddressOrStateKey::Address(*address)
                } else {
                    AddressOrStateKey::StateKey(state_key_h256)
                };

            let account_output = account_trie_record_to_output(state, account);
            (addr_or_state_key, account_output)
        })
        .collect();

    GenerationOutputs { accounts }
}

fn account_trie_record_to_output<F: Field>(
    state: &GenerationState<F>,
    account: AccountTrieRecord,
) -> AccountOutput {
    let storage = get_storage(state, account.storage_ptr);

    // TODO: This won't work if the account was created during the txn.
    // Need to track changes to code, similar to how we track addresses
    // with observe_new_address.
    let code = state
        .inputs
        .contract_code
        .get(&account.code_hash)
        .unwrap_or_else(|| panic!("Code not found: {:?}", account.code_hash))
        .clone();

    AccountOutput {
        balance: account.balance,
        nonce: account.nonce,
        storage,
        code,
    }
}

/// Get an account's storage trie, given a pointer to its root.
fn get_storage<F: Field>(state: &GenerationState<F>, storage_ptr: usize) -> HashMap<U256, U256> {
    read_trie::<U256>(&state.memory, storage_ptr, read_storage_trie_value)
        .into_iter()
        .map(|(storage_key_nibbles, value)| {
            assert_eq!(
                storage_key_nibbles.count, 64,
                "Each storage key should have 64 nibbles = 256 bits"
            );
            (storage_key_nibbles.packed, value)
        })
        .collect()
}
