// Delete from an extension node.
// Algorithm is roughly:
//      - Let `k = length(node)`
//      - Delete `(num_nibbles-k, key[k:])` from `node.child`.
//      - If the returned child node is a branch node, the current node is replaced with an extension node with updated child.
//      - If the returned child node is an extension node, we merge the two extension nodes into one extension node.
//      - If the returned child node is a leaf node, we merge the two nodes into one leaf node.
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
    DUP1 %add_const(2) %mload_trie_data
    %stack (node_child_ptr, node_payload_ptr, key, num_nibbles, node_len, node_key, retdest) ->
        (node_child_ptr, num_nibbles, key, after_mpt_delete_extension, node_payload_ptr, node_len, node_key, retdest)
    %jump(mpt_delete)

after_mpt_delete_extension:
    // stack: updated_child_node_ptr, node_payload_ptr, node_len, node_key, retdest
    DUP1 %mload_trie_data
    // stack: child_type, updated_child_node_ptr, node_payload_ptr, node_len, node_key, retdest
    DUP1 %eq_const(@MPT_NODE_BRANCH)    %jumpi(after_mpt_delete_extension_branch)
    DUP1 %eq_const(@MPT_NODE_EXTENSION) %jumpi(after_mpt_delete_extension_extension)
    DUP1 %eq_const(@MPT_NODE_LEAF)      %jumpi(after_mpt_delete_extension_leaf)
         %eq_const(@MPT_NODE_EMPTY)     %jumpi(panic) // This should never happen.
    PANIC

after_mpt_delete_extension_branch:
    // stack: child_type, updated_child_node_ptr, node_payload_ptr, node_len, node_key, retdest
    POP
    // stack: updated_child_node_ptr, node_payload_ptr, node_len, node_key, retdest
    DUP2 %add_const(2) %mstore_trie_data
    // stack: node_payload_ptr, node_len, node_key, retdest
    %decrement
    %stack (extension_ptr, node_len, node_key, retdest) -> (retdest, extension_ptr)
    JUMP

after_mpt_delete_extension_extension:
    // stack: child_type, updated_child_node_ptr, node_payload_ptr, node_len, node_key, retdest
    POP SWAP1 POP
    // stack: updated_child_node_ptr, node_len, node_key, retdest
    DUP1 %increment %mload_trie_data
    // stack: child_len, updated_child_node_ptr, node_len, node_key, retdest
    DUP2 %add_const(2) %mload_trie_data
    // stack: child_key, child_len, updated_child_node_ptr, node_len, node_key, retdest
    %stack (child_key, child_len, updated_child_node_ptr, node_len, node_key) -> (node_len, node_key, child_len, child_key, updated_child_node_ptr)
    %merge_nibbles
    // stack: len, key, updated_child_node_ptr, retdest
    DUP3 %increment %mstore_trie_data // Change len
    // stack: key, updated_child_node_ptr, retdest
    DUP2 %add_const(2) %mstore_trie_data // Change key
    // stack: extension_ptr, retdest
    SWAP1 JUMP

// Essentially the same as `after_mpt_delete_extension_extension`. TODO: Could merge in a macro or common function.
after_mpt_delete_extension_leaf:
    // stack: child_type, updated_child_node_ptr, node_payload_ptr, node_len, node_key, retdest
    POP SWAP1 POP
    // stack: updated_child_node_ptr, node_len, node_key, retdest
    DUP1 %increment %mload_trie_data
    // stack: child_len, updated_child_node_ptr, node_len, node_key, retdest
    DUP2 %add_const(2) %mload_trie_data
    // stack: child_key, child_len, updated_child_node_ptr, node_len, node_key, retdest
    %stack (child_key, child_len, updated_child_node_ptr, node_len, node_key) -> (node_len, node_key, child_len, child_key, updated_child_node_ptr)
    %merge_nibbles
    // stack: len, key, updated_child_node_ptr, retdest
    DUP3 %increment %mstore_trie_data // Change len
    // stack: key, updated_child_node_ptr, retdest
    DUP2 %add_const(2) %mstore_trie_data // Change key
    // stack: updated_child_node_ptr, retdest
    SWAP1 JUMP
