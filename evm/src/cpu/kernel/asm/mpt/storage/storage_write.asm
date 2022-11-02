// Write a word to the current account's storage trie.
//
// Pre stack: slot, value, retdest
// Post stack: (empty)

global storage_write:
    // TODO: If value = 0, delete the key instead of inserting 0.
    // stack: slot, value, retdest

    // First we write the value to MPT data, and get a pointer to it.
    %get_trie_data_size
    // stack: value_ptr, slot, value, retdest
    SWAP2
    // stack: value, slot, value_ptr, retdest
    %append_to_trie_data
    // stack: slot, value_ptr, retdest

    // Next, call mpt_insert on the current account's storage root.
    %stack (slot, value_ptr) -> (slot, value_ptr, after_storage_insert)
    %slot_to_storage_key
    // stack: storage_key, value_ptr, after_storage_write, retdest
    PUSH 64 // storage_key has 64 nibbles
    %current_storage_trie
    // stack: storage_root_ptr, 64, storage_key, value_ptr, after_storage_insert, retdest
    %jump(mpt_insert)

after_storage_insert:
    // stack: new_storage_root_ptr, retdest
    %current_account_data
    // stack: old_account_ptr, new_storage_root_ptr, retdest
    %make_account_copy
    // stack: new_account_ptr, new_storage_root_ptr, retdest

    // Update the copied account with our new storage root pointer.
    %stack (new_account_ptr, new_storage_root_ptr) -> (new_account_ptr, new_storage_root_ptr, new_account_ptr)
    %add_const(2)
    // stack: new_account_storage_root_ptr_ptr, new_storage_root_ptr, new_account_ptr, retdest
    %mstore_trie_data
    // stack: new_account_ptr, retdest

    // Save this updated account to the state trie.
    %address %addr_to_state_key
    // stack: state_key, new_account_ptr, retdest
    %jump(mpt_insert_state_trie)
