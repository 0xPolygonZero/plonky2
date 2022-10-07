// Given an address, return a pointer to the associated account data, which
// consists of four words (nonce, balance, storage_root, code_hash), in the
// state trie. Returns 0 if the address is not found.
global mpt_read_state_trie:
    // stack: addr, retdest
    // The key is the hash of the address. Since KECCAK_GENERAL takes input from
    // memory, we will write addr bytes to SEGMENT_KERNEL_GENERAL[0..20] first.
    %stack (addr) -> (0, @SEGMENT_KERNEL_GENERAL, 0, addr, 20, mpt_read_state_trie_after_mstore)
    %jump(mstore_unpacking)
mpt_read_state_trie_after_mstore:
    // stack: retdest
    %stack () -> (0, @SEGMENT_KERNEL_GENERAL, 0, 20) // context, segment, offset, len
    KECCAK_GENERAL
    // stack: key, retdest
    PUSH 64 // num_nibbles
    %mload_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT) // node_ptr
    // stack: node_ptr, num_nibbles, key, retdest
    %jump(mpt_read)

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
    SWAP1 %increment SWAP1
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
    // stack: value_len_ptr, retdest
    DUP1 %mload_trie_data
    // stack: value_len, value_len_ptr, retdest
    %jumpi(mpt_read_branch_found_value)
    // This branch node contains no value, so return null.
    %stack (value_len_ptr, retdest) -> (retdest, 0)
mpt_read_branch_found_value:
    // stack: value_len_ptr, retdest
    %increment
    // stack: value_ptr, retdest
    SWAP1
    JUMP

mpt_read_extension:
    // stack: node_type, node_payload_ptr, num_nibbles, key, retdest
    %stack (node_type, node_payload_ptr, num_nibbles, key)
        -> (num_nibbles, key, node_payload_ptr)
    // stack: num_nibbles, key, node_payload_ptr, retdest
    DUP3 %mload_trie_data
    // stack: node_num_nibbles, num_nibbles, key, node_payload_ptr, retdest
    SWAP1
    SUB
    // stack: future_nibbles, key, node_payload_ptr, retdest
    DUP2 DUP2
    // stack: future_nibbles, key, future_nibbles, key, node_payload_ptr, retdest
    %mul_const(4) SHR // key_part = key >> (future_nibbles * 4)
    DUP1
    // stack: key_part, key_part, future_nibbles, key, node_payload_ptr, retdest
    DUP5 %increment %mload_trie_data
    // stack: node_key, key_part, key_part, future_nibbles, key, node_payload_ptr, retdest
    EQ // does the first part of our key match the node's key?
    %jumpi(mpt_read_extension_found)
    // Not found; return 0.
    %stack (key_part, future_nibbles, node_payload_ptr, retdest) -> (retdest, 0)
    JUMP
mpt_read_extension_found:
    // stack: key_part, future_nibbles, key, node_payload_ptr, retdest
    DUP2 %mul_const(4) SHL // key_part_shifted = (key_part << (future_nibbles * 4))
    // stack: key_part_shifted, future_nibbles, key, node_payload_ptr, retdest
    %stack (key_part_shifted, future_nibbles, key)
        -> (key, key_part_shifted, future_nibbles)
    SUB // key -= key_part_shifted
    // stack: key, future_nibbles, node_payload_ptr, retdest
    SWAP2
    // stack: node_payload_ptr, future_nibbles, key, retdest
    %add_const(2) // child pointer is third field of extension node
    %mload_trie_data
    // stack: child_ptr, future_nibbles, key, retdest
    %jump(mpt_read) // recurse

mpt_read_leaf:
    // stack: node_type, node_payload_ptr, num_nibbles, key, retdest
    POP
    // stack: node_payload_ptr, num_nibbles, key, retdest
    DUP1 %mload_trie_data
    // stack: node_num_nibbles, node_payload_ptr, num_nibbles, key, retdest
    DUP2 %increment %mload_trie_data
    // stack: node_key, node_num_nibbles, node_payload_ptr, num_nibbles, key, retdest
    SWAP3
    // stack: num_nibbles, node_num_nibbles, node_payload_ptr, node_key, key, retdest
    EQ
    %stack (num_nibbles_match, node_payload_ptr, node_key, key)
        -> (key, node_key, num_nibbles_match, node_payload_ptr)
    EQ
    AND
    // stack: keys_match && num_nibbles_match, node_payload_ptr, retdest
    %jumpi(mpt_read_leaf_found)
    // Not found; return 0.
    %stack (node_payload_ptr, retdest) -> (retdest, 0)
    JUMP
mpt_read_leaf_found:
    // stack: node_payload_ptr, retdest
    %add_const(3) // The value is located after num_nibbles, the key, and the value length.
    // stack: value_ptr, retdest
    SWAP1
    JUMP
