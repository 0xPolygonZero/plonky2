// Computes the Merkle root of the given trie node.
//
// encode_value is a function which should take as input
// - the position withing @SEGMENT_RLP_RAW to write to,
// - the offset of a value within @SEGMENT_TRIE_DATA, and
// - a return address.
// It should serialize the value, write it to @SEGMENT_RLP_RAW starting at the
// given position, and return an updated position (the next unused offset).
//
// Pre stack: node_ptr, encode_value, retdest
// Post stack: hash
global mpt_hash:
    // stack: node_ptr, encode_value, retdest
    %stack (node_ptr, encode_value) -> (node_ptr, encode_value, mpt_hash_hash_if_rlp)
    %jump(encode_or_hash_node)
mpt_hash_hash_if_rlp:
    // stack: result, result_len, retdest
    // If result_len < 32, then we have an RLP blob, and we need to hash it.
    DUP2 %lt_const(32) %jumpi(mpt_hash_hash_rlp)
    // Otherwise, we already have a hash, so just return it.
    // stack: result, result_len, retdest
    %stack (result, result_len, retdest) -> (retdest, result)
    JUMP
mpt_hash_hash_rlp:
    // stack: result, result_len, retdest
    %stack (result, result_len)
        // context, segment, offset, value, len, retdest
        -> (0, @SEGMENT_RLP_RAW, 0, result, result_len, mpt_hash_hash_rlp_after_unpacking)
    %jump(mstore_unpacking)
mpt_hash_hash_rlp_after_unpacking:
    // stack: result_len, retdest
    PUSH 0 // offset
    PUSH @SEGMENT_RLP_RAW // segment
    PUSH 0 // context
    // stack: result_addr: 3, result_len, retdest
    KECCAK_GENERAL
    // stack: hash, retdest
    SWAP1
    JUMP

// Given a trie node, return its RLP encoding if it is is less than 32 bytes,
// otherwise return the Keccak256 hash of its RLP encoding.
//
// The result is given as a (value, length) pair, where the length is given
// in bytes.
//
// Pre stack: node_ptr, encode_value, retdest
// Post stack: result, result_len
global encode_or_hash_node:
    %stack (node_ptr, encode_value) -> (node_ptr, encode_value, maybe_hash_node)
    %jump(encode_node)
maybe_hash_node:
    // stack: result_ptr, result_len, retdest
    DUP2 %lt_const(32)
    %jumpi(pack_small_rlp)

    // result_len >= 32, so we hash the result.
    // stack: result_ptr, result_len, retdest
    PUSH @SEGMENT_RLP_RAW // segment
    PUSH 0 // context
    // stack: result_addr: 3, result_len, retdest
    KECCAK_GENERAL
    %stack (hash, retdest) -> (retdest, hash, 32)
    JUMP
pack_small_rlp:
    // stack: result_ptr, result_len, retdest
    PANIC // TODO: Return packed RLP

// RLP encode the given trie node, and return an (pointer, length) pair
// indicating where the data lives within @SEGMENT_RLP_RAW.
//
// Pre stack: node_ptr, encode_value, retdest
// Post stack: result_ptr, result_len
global encode_node:
    // stack: node_ptr, encode_value, retdest
    DUP1 %mload_trie_data
    // stack: node_type, node_ptr, encode_value, retdest
    // Increment node_ptr, so it points to the node payload instead of its type.
    SWAP1 %add_const(1) SWAP1
    // stack: node_type, node_payload_ptr, encode_value, retdest

    DUP1 %eq_const(@MPT_NODE_EMPTY)     %jumpi(encode_node_empty)
    DUP1 %eq_const(@MPT_NODE_HASH)      %jumpi(encode_node_hash)
    DUP1 %eq_const(@MPT_NODE_BRANCH)    %jumpi(encode_node_branch)
    DUP1 %eq_const(@MPT_NODE_EXTENSION) %jumpi(encode_node_extension)
    DUP1 %eq_const(@MPT_NODE_LEAF)      %jumpi(encode_node_leaf)
    PANIC // Invalid node type? Shouldn't get here.

global encode_node_empty:
    // stack: node_type, node_payload_ptr, encode_value, retdest
    %pop3
    // stack: retdest
    // An empty node is encoded as a single byte, 0x80, which is the RLP
    // encoding of the empty string. Write this byte to RLP[0] and return
    // (0, 1).
    PUSH 0x80
    PUSH 0
    %mstore_rlp
    %stack (retdest) -> (retdest, 0, 1)
    JUMP

global encode_node_hash:
    // stack: node_type, node_payload_ptr, encode_value, retdest
    POP
    // stack: node_payload_ptr, encode_value, retdest
    %mload_trie_data
    %stack (hash, encode_value, retdest) -> (retdest, hash, 32)
    JUMP

encode_node_branch:
    // stack: node_type, node_payload_ptr, encode_value, retdest
    POP
    // stack: node_payload_ptr, encode_value, retdest
    PUSH 9 // rlp_pos; we start at 9 to leave room to prepend a list prefix
    %rep 16
        // stack: rlp_pos, node_child_ptr, encode_value, retdest
        // TODO: Append encode_or_hash_node(child) to our RLP. Do all encode_or_hash_node calls first to avoid clobbering.
        SWAP1 %increment SWAP1 // node_child_ptr += 1
    %endrep
    // stack: node_value_ptr, encode_value, retdest
    PANIC // TODO

encode_node_extension:
    // stack: node_type, node_payload_ptr, encode_value, retdest
    POP
    // stack: node_payload_ptr, encode_value, retdest
    PANIC // TODO

encode_node_leaf:
    // stack: node_type, node_payload_ptr, encode_value, retdest
    POP
    // stack: node_payload_ptr, encode_value, retdest
    PUSH encode_node_leaf_after_hex_prefix // retdest
    PUSH 1 // terminated
    // stack: terminated, encode_node_leaf_after_hex_prefix, node_payload_ptr, encode_value, retdest
    DUP3 %add_const(1) %mload_trie_data // Load the packed_nibbles field, which is at index 1.
    // stack: packed_nibbles, terminated, encode_node_leaf_after_hex_prefix, node_payload_ptr, encode_value, retdest
    DUP4 %mload_trie_data // Load the num_nibbles field, which is at index 0.
    // stack: num_nibbles, packed_nibbles, terminated, encode_node_leaf_after_hex_prefix, node_payload_ptr, encode_value, retdest
    PUSH 9 // We start at 9 to leave room to prepend the largest possible RLP list header.
    // stack: rlp_start, num_nibbles, packed_nibbles, terminated, encode_node_leaf_after_hex_prefix, node_payload_ptr, encode_value, retdest
    %jump(hex_prefix_rlp)
encode_node_leaf_after_hex_prefix:
    // stack: rlp_pos, node_payload_ptr, encode_value, retdest
    SWAP1
    %add_const(2) // The value starts at index 2.
    // stack: value_ptr, rlp_pos, encode_value, retdest
    %stack (value_ptr, rlp_pos, encode_value, retdest)
        -> (encode_value, rlp_pos, value_ptr, encode_node_leaf_after_encode_value, retdest)
    JUMP
encode_node_leaf_after_encode_value:
    // stack: rlp_end_pos, retdest
    %prepend_rlp_list_prefix
    %stack (rlp_start_pos, rlp_len, retdest) -> (retdest, rlp_start_pos, rlp_len)
    JUMP
