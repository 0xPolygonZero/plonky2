// Read a value from a MPT.
//
// Arguments:
// - the virtual address of the trie to search in
// - the key, as a U256
// - the number of nibbles in the key
//
// This function returns a pointer to the leaf, or 0 if the key is not found.

global mpt_read:
    // stack: node_ptr, key, nibbles, retdest
    DUP1
    %mload_trie_data
    // stack: node_type, node_ptr, key, nibbles, retdest
    // Increment node_ptr, so it points to the node payload instead of its type.
    SWAP1 %add_const(1) SWAP1
    // stack: node_type, node_payload_ptr, key, nibbles, retdest

    DUP1 %eq_const(@MPT_NODE_EMPTY)     %jumpi(mpt_read_empty)
    DUP1 %eq_const(@MPT_NODE_BRANCH)    %jumpi(mpt_read_branch)
    DUP1 %eq_const(@MPT_NODE_EXTENSION) %jumpi(mpt_read_extension)
    DUP1 %eq_const(@MPT_NODE_LEAF)      %jumpi(mpt_read_leaf)

    // There's still the MPT_NODE_HASH case, but if we hit a digest node,
    // it means the prover failed to provide necessary Merkle data, so panic.
    PANIC

mpt_read_empty:
    // Return 0 to indicate that the value was not found.
    %stack (node_type, node_payload_ptr, key, nibbles, retdest)
        -> (retdest, 0)
    JUMP

mpt_read_branch:
    // stack: node_type, node_payload_ptr, key, nibbles, retdest
    POP
    // stack: node_payload_ptr, key, nibbles, retdest
    DUP3 // nibbles
    ISZERO
    // stack: nibbles == 0, node_payload_ptr, key, nibbles, retdest
    %jumpi(mpt_read_branch_end_of_key)

    // stack: node_payload_ptr, key, nibbles, retdest
    // We have not reached the end of the key, so we descend to one of our children.
    // Decrement nibbles, then compute current_nibble = (key >> (nibbles * 4)) & 0xF.
    SWAP2
    %sub_const(1)
    // stack: nibbles, key, node_payload_ptr, retdest
    DUP2 DUP2
    // stack: nibbles, key, nibbles, key, node_payload_ptr, retdest
    %mul_const(4)
    // stack: nibbles * 4, key, nibbles, key, node_payload_ptr, retdest
    SHR
    // stack: key >> (nibbles * 4), nibbles, key, node_payload_ptr, retdest
    %and_const(0xF)
    // stack: current_nibble, nibbles, key, node_payload_ptr, retdest
    %stack (current_nibble, nibbles, key, node_payload_ptr, retdest)
        -> (current_nibble, node_payload_ptr, key, nibbles, retdest)
    // child_ptr = load(node_payload_ptr + current_nibble)
    ADD
    %mload_trie_data
    // stack: child_ptr, key, nibbles, retdest
    %jump(mpt_read) // recurse

mpt_read_branch_end_of_key:
    %stack (node_payload_ptr, key, nibbles, retdest) -> (node_payload_ptr, retdest)
    // stack: node_payload_ptr, retdest
    %add_const(16) // skip over the 16 child nodes
    // stack: leaf_ptr, retdest
    SWAP1
    JUMP

mpt_read_extension:
    // stack: node_type, node_payload_ptr, key, nibbles, retdest
    POP
    // stack: node_payload_ptr, key, nibbles, retdest

mpt_read_leaf:
    // stack: node_type, node_payload_ptr, key, nibbles, retdest
    POP
    // stack: node_payload_ptr, key, nibbles, retdest
