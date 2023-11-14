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
    DUP2 %smt_read_state
    DUP1 ISZERO %jumpi(panic)
    // stack: account_ptr, storage_key, address, prev_value, retdest
    %add_const(2)
    // stack: storage_root_ptr_ptr, storage_key, address, prev_value, retdest
    %mload_trie_data
    // stack: storage_root_ptr, storage_key, address, prev_value, retdest
    %get_trie_data_size
    // stack: prev_value_ptr, storage_root_ptr, storage_key, address, prev_value, retdest
    PUSH 0 %append_to_trie_data
    DUP5 %append_to_trie_data
    %stack (prev_value_ptr, storage_root_ptr, storage_key, address, prev_value, retdest) ->
        (storage_root_ptr, storage_key, prev_value_ptr, new_storage_root, address, retdest)
    %jump(smt_insert)

delete:
    // stack: address, slot, prev_value, retdest
    SWAP2 POP
    %stack (slot, address, retdest) -> (slot, new_storage_root, address, retdest)
    %slot_to_storage_key
    // stack: storage_key, new_storage_root, address, retdest
    DUP3 %smt_read_state
    DUP1 ISZERO %jumpi(panic)
    // stack: account_ptr, storage_key, new_storage_root, address, retdest
    %add_const(2)
    // stack: storage_root_ptr_ptr, storage_key, new_storage_root, address, retdest
    %mload_trie_data
    // stack: storage_root_ptr, storage_key, new_storage_root, address, retdest
    %jump(smt_delete)

new_storage_root:
    // stack: new_storage_root_ptr, address, retdest
    DUP2 %smt_read_state
    // stack: account_ptr, new_storage_root_ptr, address, retdest

    // Update account with our new storage root pointer.
    %add_const(2)
    // stack: account_storage_root_ptr_ptr, new_storage_root_ptr, address, retdest
    %mstore_trie_data
    // stack: address, retdest
    POP JUMP
