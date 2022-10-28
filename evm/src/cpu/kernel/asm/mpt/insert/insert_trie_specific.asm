// Insertion logic specific to a particular trie.

// Mutate the state trie, inserting the given key-value pair.
global mpt_insert_state_trie:
    // stack: key, value_ptr, retdest
    %stack (key, value_ptr)
        -> (key, value_ptr, mpt_insert_state_trie_save)
    PUSH 64 // num_nibbles
    %mload_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT)
    // stack: state_root_ptr, num_nibbles, key, value_ptr, mpt_insert_state_trie_save, retdest
    %jump(mpt_insert)
mpt_insert_state_trie_save:
    // stack: updated_node_ptr, retdest
    %mstore_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT)
    JUMP
