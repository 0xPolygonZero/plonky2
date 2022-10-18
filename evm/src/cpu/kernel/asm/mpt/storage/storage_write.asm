// Write a word to the current account's storage trie.
//
// Pre stack: slot, value, retdest
// Post stack: (empty)

global storage_write:
    // stack: slot, value, retdest
    // TODO: If value = 0, delete the key instead of inserting 0?
    // TODO: Do we need to write value to MPT data and insert value_ptr? Currently some logic assumes all values are pointers, but could be relaxed so a value is any single word.
    %stack (slot, value) -> (slot, value, after_storage_insert)
    %slot_to_storage_key
    // stack: storage_key, value, after_storage_write, retdest
    PUSH 64 // storage_key has 64 nibbles
    %current_storage_trie
    // stack: storage_root_ptr, 64, storage_key, value, after_storage_insert, retdest
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

    SWAP1
    JUMP
