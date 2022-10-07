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
    %stack (result_ptr, result_len)
        -> (0, @SEGMENT_RLP_RAW, result_ptr, result_len,
            after_packed_small_rlp, result_len)
    %jump(mload_packing)
after_packed_small_rlp:
    %stack (result, result_len, retdest) -> (retdest, result, result_len)
    JUMP

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
    SWAP1 %increment SWAP1
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

    // We will call encode_or_hash_node on each child. For the i'th child, we
    // will store the result in SEGMENT_KERNEL_GENERAL[i], and its length in
    // SEGMENT_KERNEL_GENERAL_2[i].
    %encode_child(0)  %encode_child(1)  %encode_child(2)  %encode_child(3)
    %encode_child(4)  %encode_child(5)  %encode_child(6)  %encode_child(7)
    %encode_child(8)  %encode_child(9)  %encode_child(10) %encode_child(11)
    %encode_child(12) %encode_child(13) %encode_child(14) %encode_child(15)
    // stack: node_payload_ptr, encode_value, retdest

    // Now, append each child to our RLP tape.
    PUSH 9 // rlp_pos; we start at 9 to leave room to prepend a list prefix
    %append_child(0)  %append_child(1)  %append_child(2)  %append_child(3)
    %append_child(4)  %append_child(5)  %append_child(6)  %append_child(7)
    %append_child(8)  %append_child(9)  %append_child(10) %append_child(11)
    %append_child(12) %append_child(13) %append_child(14) %append_child(15)

    // stack: rlp_pos', node_payload_ptr, encode_value, retdest
    SWAP1
    %add_const(16)
    // stack: value_len_ptr, rlp_pos', encode_value, retdest
    DUP1 %mload_trie_data
    // stack: value_len, value_len_ptr, rlp_pos', encode_value, retdest
    %jumpi(encode_node_branch_with_value)
    // No value; append the empty string (0x80).
    // stack: value_len_ptr, rlp_pos', encode_value, retdest
    %stack (value_len_ptr, rlp_pos, encode_value) -> (rlp_pos, 0x80, rlp_pos)
    %mstore_rlp
    // stack: rlp_pos', retdest
    %increment
    // stack: rlp_pos'', retdest
    %jump(encode_node_branch_prepend_prefix)
encode_node_branch_with_value:
    // stack: value_len_ptr, rlp_pos', encode_value, retdest
    %increment
    // stack: value_ptr, rlp_pos', encode_value, retdest
    %stack (value_ptr, rlp_pos, encode_value)
        -> (encode_value, rlp_pos, value_ptr, encode_node_branch_prepend_prefix)
    JUMP // call encode_value
encode_node_branch_prepend_prefix:
    // stack: rlp_pos'', retdest
    %prepend_rlp_list_prefix
    %stack (start_pos, rlp_len, retdest) -> (retdest, start_pos, rlp_len)
    JUMP

// Part of the encode_node_branch function. Encodes the i'th child.
// Stores the result in SEGMENT_KERNEL_GENERAL[i], and its length in
// SEGMENT_KERNEL_GENERAL_2[i].
%macro encode_child(i)
    // stack: node_payload_ptr, encode_value, retdest
    PUSH %%after_encode
    DUP3 DUP3
    // stack: node_payload_ptr, encode_value, %%after_encode, node_payload_ptr, encode_value, retdest
    %add_const($i) %mload_trie_data
    // stack: child_i_ptr, encode_value, %%after_encode, node_payload_ptr, encode_value, retdest
    %jump(encode_or_hash_node)
%%after_encode:
    // stack: result, result_len, node_payload_ptr, encode_value, retdest
    %mstore_kernel_general($i)
    %mstore_kernel_general_2($i)
    // stack: node_payload_ptr, encode_value, retdest
%endmacro

// Part of the encode_node_branch function. Appends the i'th child's RLP.
%macro append_child(i)
    // stack: rlp_pos, node_payload_ptr, encode_value, retdest
    %mload_kernel_general($i) // load result
    %mload_kernel_general_2($i) // load result_len
    // stack: result_len, result, rlp_pos, node_payload_ptr, encode_value, retdest
    // If result_len != 32, result is raw RLP, with an appropriate RLP prefix already.
    DUP1 %sub_const(32) %jumpi(%%unpack)
    // Otherwise, result is a hash, and we need to add the prefix 0x80 + 32 = 160.
    // stack: result_len, result, rlp_pos, node_payload_ptr, encode_value, retdest
    PUSH 160
    DUP4 // rlp_pos
    %mstore_rlp
    SWAP2 %increment SWAP2 // rlp_pos += 1
%%unpack:
    %stack (result_len, result, rlp_pos, node_payload_ptr, encode_value, retdest)
        -> (rlp_pos, result, result_len, %%after_unpacking, node_payload_ptr, encode_value, retdest)
    %jump(mstore_unpacking_rlp)
%%after_unpacking:
    // stack: rlp_pos', node_payload_ptr, encode_value, retdest
%endmacro

encode_node_extension:
    // stack: node_type, node_payload_ptr, encode_value, retdest
    %stack (node_type, node_payload_ptr, encode_value)
        -> (node_payload_ptr, encode_value, encode_node_extension_after_encode_child, node_payload_ptr)
    %add_const(2) %mload_trie_data
    // stack: child_ptr, encode_value, encode_node_extension_after_encode_child, node_payload_ptr, retdest
    %jump(encode_or_hash_node)
encode_node_extension_after_encode_child:
    // stack: result, result_len, node_payload_ptr, retdest
    PUSH encode_node_extension_after_hex_prefix // retdest
    PUSH 0 // terminated
    // stack: terminated, encode_node_extension_after_hex_prefix, result, result_len, node_payload_ptr, retdest
    DUP5 %increment %mload_trie_data // Load the packed_nibbles field, which is at index 1.
    // stack: packed_nibbles, terminated, encode_node_extension_after_hex_prefix, result, result_len, node_payload_ptr, retdest
    DUP6 %mload_trie_data // Load the num_nibbles field, which is at index 0.
    // stack: num_nibbles, packed_nibbles, terminated, encode_node_extension_after_hex_prefix, result, result_len, node_payload_ptr, retdest
    PUSH 9 // We start at 9 to leave room to prepend the largest possible RLP list header.
    // stack: rlp_start, num_nibbles, packed_nibbles, terminated, encode_node_extension_after_hex_prefix, result, result_len, node_payload_ptr, retdest
    %jump(hex_prefix_rlp)
encode_node_extension_after_hex_prefix:
    // stack: rlp_pos, result, result_len, node_payload_ptr, retdest
    // If result_len != 32, result is raw RLP, with an appropriate RLP prefix already.
    DUP3 %sub_const(32) %jumpi(encode_node_extension_unpack)
    // Otherwise, result is a hash, and we need to add the prefix 0x80 + 32 = 160.
    PUSH 160
    DUP2 // rlp_pos
    %mstore_rlp
    %increment // rlp_pos += 1
encode_node_extension_unpack:
    %stack (rlp_pos, result, result_len, node_payload_ptr)
        -> (rlp_pos, result, result_len, encode_node_extension_after_unpacking)
    %jump(mstore_unpacking_rlp)
encode_node_extension_after_unpacking:
    // stack: rlp_end_pos, retdest
    %prepend_rlp_list_prefix
    %stack (rlp_start_pos, rlp_len, retdest) -> (retdest, rlp_start_pos, rlp_len)
    JUMP

encode_node_leaf:
    // stack: node_type, node_payload_ptr, encode_value, retdest
    POP
    // stack: node_payload_ptr, encode_value, retdest
    PUSH encode_node_leaf_after_hex_prefix // retdest
    PUSH 1 // terminated
    // stack: terminated, encode_node_leaf_after_hex_prefix, node_payload_ptr, encode_value, retdest
    DUP3 %increment %mload_trie_data // Load the packed_nibbles field, which is at index 1.
    // stack: packed_nibbles, terminated, encode_node_leaf_after_hex_prefix, node_payload_ptr, encode_value, retdest
    DUP4 %mload_trie_data // Load the num_nibbles field, which is at index 0.
    // stack: num_nibbles, packed_nibbles, terminated, encode_node_leaf_after_hex_prefix, node_payload_ptr, encode_value, retdest
    PUSH 9 // We start at 9 to leave room to prepend the largest possible RLP list header.
    // stack: rlp_start, num_nibbles, packed_nibbles, terminated, encode_node_leaf_after_hex_prefix, node_payload_ptr, encode_value, retdest
    %jump(hex_prefix_rlp)
encode_node_leaf_after_hex_prefix:
    // stack: rlp_pos, node_payload_ptr, encode_value, retdest
    SWAP1
    %add_const(3) // The value starts at index 3, after num_nibbles, packed_nibbles, and value_len.
    // stack: value_ptr, rlp_pos, encode_value, retdest
    %stack (value_ptr, rlp_pos, encode_value, retdest)
        -> (encode_value, rlp_pos, value_ptr, encode_node_leaf_after_encode_value, retdest)
    JUMP
encode_node_leaf_after_encode_value:
    // stack: rlp_end_pos, retdest
    %prepend_rlp_list_prefix
    %stack (rlp_start_pos, rlp_len, retdest) -> (retdest, rlp_start_pos, rlp_len)
    JUMP
