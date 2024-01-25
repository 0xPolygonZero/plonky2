// struct StorageChange { address, slot, prev_value }

%macro journal_add_storage_change
    %journal_add_3(@JOURNAL_ENTRY_STORAGE_CHANGE)
%endmacro

global revert_storage_change:
    // stack: entry_type, ptr, retdest
    POP
    %journal_load_3
    // stack: address, slot, prev_value, retdest
    DUP3 ISZERO %jumpi(delete)
    // stack: address, slot, prev_value, retdest
    SWAP1 %slot_to_storage_key
    // stack: storage_key, address, prev_value, retdest
    PUSH 64 // storage_key has 64 nibbles
    // stack: 64, storage_key, address, prev_value, retdest
    DUP3 %mpt_read_state_trie
    DUP1 ISZERO %jumpi(panic)
    // stack: account_ptr, 64, storage_key, address, prev_value, retdest
    %add_const(2)
    // stack: storage_root_ptr_ptr, 64, storage_key, address, prev_value, retdest
    %mload_trie_data
    %get_trie_data_size
    DUP6 %append_to_trie_data
    %stack (prev_value_ptr, storage_root_ptr, num_nibbles, storage_key, address, prev_value, retdest) ->
        (storage_root_ptr, num_nibbles, storage_key, prev_value_ptr, new_storage_root, address, retdest)
    %jump(mpt_insert)

delete:
    // stack: address, slot, prev_value, retdest
    SWAP2 POP
    %stack (slot, address, retdest) -> (slot, new_storage_root, address, retdest)
    %slot_to_storage_key
    // stack: storage_key, new_storage_root, address, retdest
    PUSH 64 // storage_key has 64 nibbles
    // stack: 64, storage_key, new_storage_root, address, retdest
    DUP4 %mpt_read_state_trie
    DUP1 ISZERO %jumpi(panic)
    // stack: account_ptr, 64, storage_key, new_storage_root, address, retdest
    %add_const(2)
    // stack: storage_root_ptr_ptr, 64, storage_key, new_storage_root, address, retdest
    %mload_trie_data
    // stack: storage_root_ptr, 64, storage_key, new_storage_root, address, retdest
    %jump(mpt_delete)

new_storage_root:
    // stack: new_storage_root_ptr, address, retdest
    DUP2 %mpt_read_state_trie
    // stack: account_ptr, new_storage_root_ptr, address, retdest

    // Update account with our new storage root pointer.
    %add_const(2)
    // stack: account_storage_root_ptr_ptr, new_storage_root_ptr, address, retdest
    %mstore_trie_data
    // stack: address, retdest
    POP JUMP
