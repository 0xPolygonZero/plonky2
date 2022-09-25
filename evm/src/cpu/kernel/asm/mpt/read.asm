// Read a value from a MPT.
//
// Arguments:
// - the virtual address of the trie to search in
// - the key, as a U256
// - the number of nibbles in the key (should start at 64)
//
// This function returns a pointer to the leaf, or 0 if the key is not found.

global mpt_read:
    // stack: node_ptr, num_nibbles, key, retdest
    DUP1
    %mload_trie_data
    // stack: node_type, node_ptr, num_nibbles, key, retdest
    // Increment node_ptr, so it points to the node payload instead of its type.
    SWAP1 %add_const(1) SWAP1
    // stack: node_type, node_payload_ptr, num_nibbles, key, retdest

    DUP1 %eq_const(@MPT_NODE_EMPTY)     %jumpi(mpt_read_empty)
    DUP1 %eq_const(@MPT_NODE_BRANCH)    %jumpi(mpt_read_branch)
    DUP1 %eq_const(@MPT_NODE_EXTENSION) %jumpi(mpt_read_extension)
    DUP1 %eq_const(@MPT_NODE_LEAF)      %jumpi(mpt_read_leaf)

    // There's still the MPT_NODE_HASH case, but if we hit a digest node,
    // it means the prover failed to provide necessary Merkle data, so panic.
    PANIC

mpt_read_empty:
    // Return 0 to indicate that the value was not found.
    %stack (node_type, node_payload_ptr, num_nibbles, key, retdest)
        -> (retdest, 0)
    JUMP

mpt_read_branch:
    // stack: node_type, node_payload_ptr, num_nibbles, key, retdest
    POP
    // stack: node_payload_ptr, num_nibbles, key, retdest
    DUP2 // num_nibbles
    ISZERO
    // stack: num_nibbles == 0, node_payload_ptr, num_nibbles, key, retdest
    %jumpi(mpt_read_branch_end_of_key)

    // We have not reached the end of the key, so we descend to one of our children.
    // stack: node_payload_ptr, num_nibbles, key, retdest
    %stack (node_payload_ptr, num_nibbles, key)
        -> (num_nibbles, key, node_payload_ptr)
    // stack: num_nibbles, key, node_payload_ptr, retdest
    %split_first_nibble
    %stack (first_nibble, num_nibbles, key, node_payload_ptr)
        -> (node_payload_ptr, first_nibble, num_nibbles, key)
    // child_ptr = load(node_payload_ptr + first_nibble)
    ADD %mload_trie_data
    // stack: child_ptr, num_nibbles, key, retdest
    %jump(mpt_read) // recurse

mpt_read_branch_end_of_key:
    %stack (node_payload_ptr, num_nibbles, key, retdest) -> (node_payload_ptr, retdest)
    // stack: node_payload_ptr, retdest
    %add_const(16) // skip over the 16 child nodes
    // stack: leaf_ptr, retdest
    SWAP1
    JUMP

mpt_read_extension:
    // stack: node_type, node_payload_ptr, num_nibbles, key, retdest
    POP
    // stack: node_payload_ptr, num_nibbles, key, retdest
    // TODO

mpt_read_leaf:
    // stack: node_type, node_payload_ptr, key, num_nibbles, retdest
    POP
    // stack: node_payload_ptr, key, num_nibbles, retdest
    DUP1 %mload_trie_data
    // stack: node_num_nibbles, node_payload_ptr, key, num_nibbles, retdest
    DUP2 %add_const(1) %mload_trie_data
    // stack: node_key, node_num_nibbles, node_payload_ptr, key, num_nibbles, retdest
    SWAP4
    // stack: num_nibbles, node_num_nibbles, node_payload_ptr, key, node_key, retdest
    EQ
    %stack (num_nibbles_match, node_payload_ptr, key, node_key)
        -> (key, node_key, num_nibbles_match, node_payload_ptr)
    EQ
    AND
    // stack: keys_match && num_nibbles_match, node_payload_ptr, retdest
    %jumpi(mpt_read_leaf_found)
    %stack (node_payload_ptr, retdest) -> (retdest, 0)
    JUMP
mpt_read_leaf_found:
    // stack: node_payload_ptr, retdest
    %add_const(2) // The leaf data is located after num_nibbles and the key.
    // stack: value_ptr, retdest
    SWAP1
    JUMP
