global mpt_delete_extension:
    // stack: node_type, node_payload_ptr, num_nibbles, key, retdest
    POP
    // stack: node_payload_ptr, num_nibbles, key, retdest
    DUP1 %mload_trie_data
    // stack: node_len, node_payload_ptr, num_nibbles, key, retdest
    DUP2 %increment %mload_trie_data
    %stack (node_key, node_len, node_payload_ptr, num_nibbles, key, retdest) ->
        (node_len, num_nibbles, key, node_payload_ptr, node_len, node_key, retdest)
    %truncate_nibbles
    // stack: num_nibbles, key, node_payload_ptr, node_len, node_key, retdest
    SWAP2
    // stack: node_payload_ptr, key, num_nibbles, node_len, node_key, retdest
    %add_const(2) %mload_trie_data
    %stack (node_child_ptr, key, num_nibbles, node_len, node_key, retdest) ->
        (node_child_ptr, num_nibbles, key, after_mpt_delete_extension, node_len, node_key, retdest)
    %jump(mpt_delete)

after_mpt_delete_extension:
    // stack: updated_child_node_ptr, node_len, node_key, retdest
    DUP1 %mload_trie_data
    // stack: child_type, updated_child_node_ptr, node_len, node_key, retdest
    DUP1 %eq_const(@MPT_NODE_EMPTY)     %jumpi(panic) // This should never happen.
    DUP1 %eq_const(@MPT_NODE_BRANCH)    %jumpi(after_mpt_delete_extension_branch)
    DUP1 %eq_const(@MPT_NODE_EXTENSION) %jumpi(after_mpt_delete_extension_extension)
         %eq_const(@MPT_NODE_LEAF)      %jumpi(after_mpt_delete_extension_leaf)

after_mpt_delete_extension_branch:
    // stack: child_type, updated_child_node_ptr, node_len, node_key, retdest
    POP
    // stack: updated_child_node_ptr, node_len, node_key, retdest
    %get_trie_data_size // pointer to the extension node we're about to create
    // stack: extension_ptr, updated_child_node_ptr, node_len, node_key, retdest
    PUSH @MPT_NODE_EXTENSION %append_to_trie_data
    // stack: extension_ptr, updated_child_node_ptr, node_len, node_key, retdest
    SWAP2 %append_to_trie_data // Append node_len to our node
    // stack: updated_child_node_ptr, extension_ptr, node_key, retdest
    SWAP2 %append_to_trie_data // Append node_key to our node
    // stack: extension_ptr, updated_child_node_ptr, retdest
    SWAP1 %append_to_trie_data // Append updated_child_node_ptr to our node
    // stack: extension_ptr, retdest
    SWAP1 JUMP

after_mpt_delete_extension_extension:
    // stack: child_type, updated_child_node_ptr, node_len, node_key, retdest
    POP
    // stack: updated_child_node_ptr, node_len, node_key, retdest
    DUP1 %increment %mload_trie_data
    // stack: child_len, updated_child_node_ptr, node_len, node_key, retdest
    DUP2 %add_const(2) %mload_trie_data
    // stack: child_key, child_len, updated_child_node_ptr, node_len, node_key, retdest
    SWAP2 %add_const(3) %mload_trie_data
    %stack (grandchild_ptr, child_len, child_key, node_len, node_key) -> (node_len, node_key, child_len, child_key, grandchild_ptr)
    %merge_nibbles
    // stack: len, key, grandchild_ptr, retdest
    %get_trie_data_size // pointer to the extension node we're about to create
    // stack: extension_ptr, len, key, grandchild_ptr, retdest
    PUSH @MPT_NODE_EXTENSION %append_to_trie_data
    // stack: extension_ptr, len, key, grandchild_ptr, retdest
    SWAP1 %append_to_trie_data // Append len to our node
    // stack: extension_ptr, key, grandchild_ptr, retdest
    SWAP1 %append_to_trie_data // Append key to our node
    // stack: extension_ptr, grandchild_ptr, retdest
    SWAP1 %append_to_trie_data // Append grandchild_ptr to our node
    // stack: extension_ptr, retdest
    SWAP1 JUMP

// Essentially the same as `after_mpt_delete_extension_leaf`.
// TODO: Could merge in a macro.
after_mpt_delete_extension_leaf:
    // stack: updated_child_node_ptr, node_len, node_key, retdest
    DUP1 %increment %mload_trie_data
    // stack: child_len, updated_child_node_ptr, node_len, node_key, retdest
    DUP2 %add_const(2) %mload_trie_data
    // stack: child_key, child_len, updated_child_node_ptr, node_len, node_key, retdest
    SWAP2 %add_const(3) %mload_trie_data
    %stack (value_ptr, child_len, child_key, node_len, node_key) -> (node_len, node_key, child_len, child_key, value_ptr)
    %merge_nibbles
    // stack: len, key, value_ptr, retdest
    %get_trie_data_size // pointer to the leaf node we're about to create
    // stack: leaf_ptr, len, key, value_ptr, retdest
    PUSH @MPT_NODE_LEAF %append_to_trie_data
    // stack: leaf_ptr, len, key, value_ptr, retdest
    SWAP1 %append_to_trie_data // Append len to our node
    // stack: leaf_ptr, key, value_ptr, retdest
    SWAP1 %append_to_trie_data // Append key to our node
    // stack: leaf_ptr, value_ptr, retdest
    SWAP1 %append_to_trie_data // Append value_ptr to our node
    // stack: leaf_ptr, retdest
    SWAP1 JUMP
