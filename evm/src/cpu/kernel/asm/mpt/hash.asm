global mpt_hash_state_trie:
    // stack: retdest
    %mload_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT)
    // stack: node_ptr, retdest
    %mpt_hash(encode_account)

encode_account:
    // stack: rlp_pos, value_ptr, retdest
    // First, we compute the length of the RLP data we're about to write.
    // The nonce and balance fields are variable-length, so we need to load them
    // to determine their contribution, while the other two fields are fixed
    // 32-bytes integers.
    DUP2 %mload_trie_data // nonce = value[0]
    %scalar_rlp_len
    // stack: nonce_rlp_len, rlp_pos, value_ptr, retdest
    DUP3 %add_const(1) %mload_trie_data // balance = value[1]
    %scalar_rlp_len
    // stack: balance_rlp_lenm, nonce_rlp_len, rlp_pos, value_ptr, retdest
    PUSH 66 // storage_root and code_hash fields each take 1 + 32 bytes
    ADD ADD
    // stack: payload_len, rlp_pos, value_ptr, retdest
    SWAP1
    %encode_rlp_list_prefix
    // stack: rlp_pos', value_ptr, retdest
    DUP2 %mload_trie_data // nonce = value[0]
    // stack: nonce, rlp_pos', value_ptr, retdest
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos'', value_ptr, retdest
    DUP2 %add_const(1) %mload_trie_data // balance = value[1]
    // stack: balance, rlp_pos'', value_ptr, retdest
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos''', value_ptr, retdest
    DUP2 %add_const(2) %mload_trie_data // storage_root = value[2]
    // stack: storage_root, rlp_pos''', value_ptr, retdest
    SWAP1 %encode_rlp_256
    // stack: rlp_pos'''', value_ptr, retdest
    SWAP1 %add_const(3) %mload_trie_data // code_hash = value[3]
    // stack: code_hash, rlp_pos'''', retdest
    SWAP1 %encode_rlp_256
    // stack: rlp_pos''''', retdest
    SWAP1
    JUMP

// Computes the Merkle root of the given trie node.
//
// The encode_value function should take as input
// - the position withing @SEGMENT_RLP_RAW to write to,
// - the offset of a value within @SEGMENT_TRIE_DATA, and
// - a return address.
// It should serialize the value, write it to @SEGMENT_RLP_RAW starting at the
// given position, and return an updated position (the next unused offset).
%macro mpt_hash(encode_value)
    // stack: node_ptr, retdest
    DUP1
    %mload_trie_data
    // stack: node_type, node_ptr, retdest
    // Increment node_ptr, so it points to the node payload instead of its type.
    SWAP1 %add_const(1) SWAP1
    // stack: node_type, node_payload_ptr, retdest

    DUP1 %eq_const(@MPT_NODE_EMPTY)     %jumpi(mpt_hash_empty)
    DUP1 %eq_const(@MPT_NODE_HASH)      %jumpi(mpt_hash_hash)
    DUP1 %eq_const(@MPT_NODE_BRANCH)    %jumpi(%%mpt_hash_branch)
    DUP1 %eq_const(@MPT_NODE_EXTENSION) %jumpi(%%mpt_hash_extension)
    DUP1 %eq_const(@MPT_NODE_LEAF)      %jumpi(%%mpt_hash_leaf)
    PANIC // Invalid node type? Shouldn't get here.

%%mpt_hash_branch:
    // stack: node_type, node_payload_ptr, retdest
    POP
    // stack: node_payload_ptr, retdest
    PANIC // TODO

%%mpt_hash_extension:
    // stack: node_type, node_payload_ptr, retdest
    POP
    // stack: node_payload_ptr, retdest
    PANIC // TODO

%%mpt_hash_leaf:
    // stack: node_type, node_payload_ptr, retdest
    POP
    // stack: node_payload_ptr, retdest
    PUSH %%mpt_hash_leaf_after_hex_prefix // retdest
    PUSH 1 // terminated
    // stack: terminated, %%mpt_hash_leaf_after_hex_prefix, node_payload_ptr, retdest
    DUP3 %add_const(1) %mload_trie_data // Load the packed_nibbles field, which is at index 1.
    // stack: packed_nibbles, terminated, %%mpt_hash_leaf_after_hex_prefix, node_payload_ptr, retdest
    DUP4 %mload_trie_data // Load the num_nibbles field, which is at index 0.
    // stack: num_nibbles, packed_nibbles, terminated, %%mpt_hash_leaf_after_hex_prefix, node_payload_ptr, retdest
    PUSH 9 // We start at 9 to leave room to prepend the largest possible RLP list header.
    // stack: rlp_start, num_nibbles, packed_nibbles, terminated, %%mpt_hash_leaf_after_hex_prefix, node_payload_ptr, retdest
    %jump(hex_prefix_rlp)
%%mpt_hash_leaf_after_hex_prefix:
    // stack: rlp_pos, node_payload_ptr, retdest
    SWAP1
    %add_const(2) // The value starts at index 2.
    %stack (value_ptr, rlp_pos, retdest)
        -> (rlp_pos, value_ptr, %%mpt_hash_leaf_after_encode_value, retdest)
    %jump($encode_value)
%%mpt_hash_leaf_after_encode_value:
    // stack: rlp_end_pos, retdest
    %prepend_rlp_list_prefix
    // stack: rlp_start_pos, rlp_len, retdest
    PUSH @SEGMENT_RLP_RAW
    PUSH 0 // kernel context
    // stack: rlp_start_addr: 3, rlp_len, retdest
    KECCAK_GENERAL
    // stack: hash, retdest
    SWAP1
    JUMP
%endmacro

mpt_hash_empty:
    %stack (node_type, node_payload_ptr, retdest) -> (retdest, @EMPTY_NODE_HASH)
    JUMP

mpt_hash_hash:
    // stack: node_type, node_payload_ptr, retdest
    POP
    // stack: node_payload_ptr, retdest
    %mload_trie_data
    // stack: hash, retdest
    SWAP1
    JUMP
