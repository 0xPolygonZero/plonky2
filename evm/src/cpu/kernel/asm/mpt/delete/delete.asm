// Return a copy of the given node with the given key deleted.
// Assumes that the key is in the trie.
//
// Pre stack: node_ptr, num_nibbles, key, retdest
// Post stack: updated_node_ptr
global mpt_delete:
    // stack: node_ptr, num_nibbles, key, retdest
    DUP1 %mload_trie_data
    // stack: node_type, node_ptr, num_nibbles, key, retdest
    // Increment node_ptr, so it points to the node payload instead of its type.
    SWAP1 %increment SWAP1
    // stack: node_type, node_payload_ptr, num_nibbles, key, retdest

    DUP1 %eq_const(@MPT_NODE_EMPTY)     %jumpi(panic) // This should never happen.
    DUP1 %eq_const(@MPT_NODE_BRANCH)    %jumpi(mpt_delete_branch)
    DUP1 %eq_const(@MPT_NODE_EXTENSION) %jumpi(mpt_delete_extension)
         %eq_const(@MPT_NODE_LEAF)      %jumpi(mpt_delete_leaf)
    PANIC

mpt_delete_leaf:
    // stack: node_payload_ptr, num_nibbles, key, retdest
    %pop3
    PUSH 0 // empty node ptr
    SWAP1 JUMP
