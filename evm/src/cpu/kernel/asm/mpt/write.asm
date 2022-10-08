// TODO: Need a special case for deleting, if value = ''.
// Or canonicalize once, before final hashing, to remove empty leaves etc.

// Return a copy of the given node, with the given key set to the given value.
//
// Pre stack: node_ptr, num_nibbles, key, value_ptr, retdest
// Post stack: updated_node_ptr
global mpt_insert:
    // stack: node_ptr, num_nibbles, key, value_ptr, retdest
    DUP1 %mload_trie_data
    // stack: node_type, node_ptr, num_nibbles, key, value_ptr, retdest
    // Increment node_ptr, so it points to the node payload instead of its type.
    SWAP1 %increment SWAP1
    // stack: node_type, node_payload_ptr, num_nibbles, key, value_ptr, retdest

    DUP1 %eq_const(@MPT_NODE_EMPTY)     %jumpi(mpt_insert_empty)
    DUP1 %eq_const(@MPT_NODE_BRANCH)    %jumpi(mpt_insert_branch)
    DUP1 %eq_const(@MPT_NODE_EXTENSION) %jumpi(mpt_insert_extension)
    DUP1 %eq_const(@MPT_NODE_LEAF)      %jumpi(mpt_insert_leaf)

    // There's still the MPT_NODE_HASH case, but if we hit a hash node,
    // it means the prover failed to provide necessary Merkle data, so panic.
    PANIC

mpt_insert_empty:
    // stack: node_type, node_payload_ptr, num_nibbles, key, value_ptr, retdest
    POP
    // stack: node_payload_ptr, num_nibbles, key, value_ptr, retdest
    PANIC // TODO

mpt_insert_branch:
    // stack: node_type, node_payload_ptr, num_nibbles, key, value_ptr, retdest
    POP
    // stack: node_payload_ptr, num_nibbles, key, value_ptr, retdest
    PANIC // TODO

mpt_insert_extension:
    // stack: node_type, node_payload_ptr, num_nibbles, key, value_ptr, retdest
    POP
    // stack: node_payload_ptr, num_nibbles, key, value_ptr, retdest
    PANIC // TODO

mpt_insert_leaf:
    // stack: node_type, node_payload_ptr, num_nibbles, key, value_ptr, retdest
    POP
    // stack: node_payload_ptr, num_nibbles, key, value_ptr, retdest
    PANIC // TODO
