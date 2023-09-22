use std::collections::HashMap;

use ethereum_types::{Address, BigEndianHash, H256, U256};
use plonky2::field::types::Field;

use crate::cpu::kernel::constants::global_metadata::GlobalMetadata::StateTrieRoot;
use crate::generation::state::GenerationState;
use crate::generation::trie_extractor::{
    read_state_trie_value, read_storage_trie_value, read_trie, AccountTrieRecord,
};
use crate::util::u256_to_usize;
use crate::witness::errors::ProgramError;

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

pub(crate) fn get_outputs<F: Field>(
    state: &mut GenerationState<F>,
) -> Result<GenerationOutputs, ProgramError> {
    // First observe all addresses passed in by caller.
    for address in state.inputs.addresses.clone() {
        state.observe_address(address);
    }

    let ptr = u256_to_usize(state.memory.read_global_metadata(StateTrieRoot))?;
    let account_map = read_trie::<AccountTrieRecord>(&state.memory, ptr, read_state_trie_value)?;

    let mut accounts = HashMap::with_capacity(account_map.len());

    for (state_key_nibbles, account) in account_map.into_iter() {
        if state_key_nibbles.count != 64 {
            return Err(ProgramError::IntegerTooLarge);
        }
        let state_key_h256 = H256::from_uint(&state_key_nibbles.try_into_u256().unwrap());

        let addr_or_state_key =
            if let Some(address) = state.state_key_to_address.get(&state_key_h256) {
                AddressOrStateKey::Address(*address)
            } else {
                AddressOrStateKey::StateKey(state_key_h256)
            };

        let account_output = account_trie_record_to_output(state, account)?;
        accounts.insert(addr_or_state_key, account_output);
    }

    Ok(GenerationOutputs { accounts })
}

fn account_trie_record_to_output<F: Field>(
    state: &GenerationState<F>,
    account: AccountTrieRecord,
) -> Result<AccountOutput, ProgramError> {
    let storage = get_storage(state, account.storage_ptr)?;

    // TODO: This won't work if the account was created during the txn.
    // Need to track changes to code, similar to how we track addresses
    // with observe_new_address.
    let code = state
        .inputs
        .contract_code
        .get(&account.code_hash)
        .ok_or_else(|| ProgramError::UnknownContractCode)?
        .clone();

    Ok(AccountOutput {
        balance: account.balance,
        nonce: account.nonce,
        storage,
        code,
    })
}

/// Get an account's storage trie, given a pointer to its root.
fn get_storage<F: Field>(
    state: &GenerationState<F>,
    storage_ptr: usize,
) -> Result<HashMap<U256, U256>, ProgramError> {
    let storage_trie = read_trie::<U256>(&state.memory, storage_ptr, |x| {
        Ok(read_storage_trie_value(x))
    })?;

    let mut map = HashMap::with_capacity(storage_trie.len());
    for (storage_key_nibbles, value) in storage_trie.into_iter() {
        if storage_key_nibbles.count != 64 {
            return Err(ProgramError::IntegerTooLarge);
        };
        map.insert(storage_key_nibbles.try_into_u256().unwrap(), value);
    }

    Ok(map)
}
