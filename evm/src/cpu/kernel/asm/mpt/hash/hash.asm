// Computes the Merkle root of the given trie node.
//
// encode_value is a function which should take as input
// - the position within @SEGMENT_RLP_RAW to write to,
// - the offset of a value within @SEGMENT_TRIE_DATA,
// - a return address, and
// - the current length of @SEGMENT_TRIE_DATA
// It should serialize the value, write it to @SEGMENT_RLP_RAW starting at the
// given position, and return an updated position (the next unused offset) as well
// as an updated length for @SEGMENT_TRIE_DATA.
//
// Given the initial length of the `TrieData` segment, it also updates the length
// for the current trie.
//
// Pre stack: node_ptr, encode_value, cur_len, retdest
// Post stack: hash, new_len
global mpt_hash:
    // stack: node_ptr, encode_value, cur_len, retdest
    %stack (node_ptr, encode_value, cur_len) -> (node_ptr, encode_value, cur_len, mpt_hash_hash_if_rlp)
    %jump(encode_or_hash_node)
mpt_hash_hash_if_rlp:
    // stack: result, result_len, new_len, retdest
    // If result_len < 32, then we have an RLP blob, and we need to hash it.
    DUP2 %lt_const(32) %jumpi(mpt_hash_hash_rlp)
    // Otherwise, we already have a hash, so just return it.
    // stack: result, result_len, new_len, retdest
    %stack (result, result_len, new_len, retdest) -> (retdest, result, new_len)
    JUMP
mpt_hash_hash_rlp:
    // stack: result, result_len, new_len, retdest
    %stack (result, result_len, new_len)
        -> (@SEGMENT_RLP_RAW, result, result_len, mpt_hash_hash_rlp_after_unpacking, result_len, new_len)
    // stack: addr, result, result_len, mpt_hash_hash_rlp_after_unpacking, result_len, new_len
    %jump(mstore_unpacking)
mpt_hash_hash_rlp_after_unpacking:
    // stack: result_addr, result_len, new_len, retdest
    POP PUSH @SEGMENT_RLP_RAW // ctx == virt == 0
    // stack: result_addr, result_len, new_len, retdest
    KECCAK_GENERAL
    // stack: hash, new_len, retdest
    %stack(hash, new_len, retdest) -> (retdest, hash, new_len)
    JUMP

// Given a trie node, return its RLP encoding if it is is less than 32 bytes,
// otherwise return the Keccak256 hash of its RLP encoding.
//
// The result is given as a (value, length) pair, where the length is given
// in bytes.
//
// Pre stack: node_ptr, encode_value, cur_len, retdest
// Post stack: result, result_len, cur_len
global encode_or_hash_node:
    DUP1 %mload_trie_data

    // Check if we're dealing with a concrete node, i.e. not a hash node.
    // stack: node_type, node_ptr, encode_value, cur_len, retdest
    DUP1
    PUSH @MPT_NODE_HASH
    SUB
    %jumpi(encode_or_hash_concrete_node)

    // If we got here, node_type == @MPT_NODE_HASH.
    // Load the hash and return (hash, 32).
    // stack: node_type, node_ptr, encode_value, cur_len, retdest
    POP
    // Update the length of the `TrieData` segment: there are only two 
    // elements in a hash node.
    SWAP2 %add_const(2) SWAP2
    // stack: node_ptr, encode_value, cur_len, retdest
    %increment // Skip over node type prefix
    // stack: hash_ptr, encode_value, cur_len, retdest
    %mload_trie_data
    // stack: hash, encode_value, cur_len, retdest
    %stack (hash, encode_value, cur_len, retdest) -> (retdest, hash, 32, cur_len)
    JUMP
encode_or_hash_concrete_node:
    %stack (node_type, node_ptr, encode_value, cur_len) -> (node_type, node_ptr, encode_value, cur_len, maybe_hash_node)
    %jump(encode_node)
maybe_hash_node:
    // stack: result_addr, result_len, cur_len, retdest
    DUP2 %lt_const(32)
    %jumpi(pack_small_rlp)

    // result_len >= 32, so we hash the result.
    // stack: result_addr, result_len, cur_len, retdest
    KECCAK_GENERAL
    %stack (hash, cur_len, retdest) -> (retdest, hash, 32, cur_len)
    JUMP
pack_small_rlp:
    // stack: result_ptr, result_len, cur_len, retdest
    %stack (result_ptr, result_len, cur_len)
        -> (result_ptr, result_len, after_packed_small_rlp, result_len, cur_len)
    %jump(mload_packing)
after_packed_small_rlp:
    %stack (result, result_len, cur_len, retdest) -> (retdest, result, result_len, cur_len)
    JUMP

// RLP encode the given trie node, and return an (pointer, length) pair
// indicating where the data lives within @SEGMENT_RLP_RAW.
//
// Pre stack: node_type, node_ptr, encode_value, cur_len, retdest
// Post stack: result_ptr, result_len, cur_len
encode_node:
    // stack: node_type, node_ptr, encode_value, cur_len, retdest
    // Increment node_ptr, so it points to the node payload instead of its type.
    SWAP1 %increment SWAP1
    // stack: node_type, node_payload_ptr, encode_value, cur_len, retdest

    DUP1 %eq_const(@MPT_NODE_EMPTY)     %jumpi(encode_node_empty)
    DUP1 %eq_const(@MPT_NODE_BRANCH)    %jumpi(encode_node_branch)
    DUP1 %eq_const(@MPT_NODE_EXTENSION) %jumpi(encode_node_extension)
    DUP1 %eq_const(@MPT_NODE_LEAF)      %jumpi(encode_node_leaf)

    // If we got here, node_type is either @MPT_NODE_HASH, which should have
    // been handled earlier in encode_or_hash_node, or something invalid.
    PANIC

global encode_node_empty:
    // stack: node_type, node_payload_ptr, encode_value, cur_len, retdest
    %pop3
    %stack (cur_len, retdest) -> (retdest, @ENCODED_EMPTY_NODE_POS, 1, cur_len)
    JUMP

global encode_node_branch:
    // stack: node_type, node_payload_ptr, encode_value, cur_len, retdest
    POP

    // `TrieData` stores the node type, 16 children pointers, and a value pointer.
    SWAP2 %add_const(18) SWAP2
    // stack: node_payload_ptr, encode_value, cur_len, retdest

    // Allocate a block of RLP memory
    %alloc_rlp_block DUP1
    // stack: rlp_pos, rlp_start, node_payload_ptr, encode_value, cur_len retdest

    // Call encode_or_hash_node on each child 
    %encode_child(0)  %encode_child(1)  %encode_child(2)  %encode_child(3)
    %encode_child(4)  %encode_child(5)  %encode_child(6)  %encode_child(7)
    %encode_child(8)  %encode_child(9)  %encode_child(10) %encode_child(11)
    %encode_child(12) %encode_child(13) %encode_child(14) %encode_child(15)

    // stack: rlp_pos', rlp_start, node_payload_ptr, encode_value, cur_len, retdest

    %stack (rlp_pos, rlp_start, node_payload_ptr)
        -> (node_payload_ptr, rlp_pos, rlp_start)
    %add_const(16)
    // stack: value_ptr_ptr, rlp_pos', rlp_start, encode_value, cur_len, retdest
    %mload_trie_data
    // stack: value_ptr, rlp_pos', rlp_start, encode_value, cur_len, retdest
    DUP1 %jumpi(encode_node_branch_with_value)

    // No value; append the empty string (0x80).
    // stack: value_ptr, rlp_pos', rlp_start, encode_value, cur_len, retdest
    %stack (value_ptr, rlp_pos, rlp_start, encode_value) -> (0x80, rlp_pos, rlp_pos, rlp_start)
    MSTORE_GENERAL
    // stack: rlp_pos', rlp_start, cur_len, retdest
    %increment
    // stack: rlp_pos'', rlp_start, cur_len, retdest
    %jump(encode_node_branch_prepend_prefix)
encode_node_branch_with_value:
    // stack: value_ptr, rlp_pos', rlp_start, encode_value, cur_len, retdest
    %stack (value_ptr, rlp_pos, rlp_start, encode_value, cur_len)
        -> (encode_value, rlp_pos, value_ptr, cur_len, encode_node_branch_after_value, rlp_start)
    JUMP // call encode_value
encode_node_branch_after_value:
    // stack: rlp_pos'', cur_len, rlp_start, retdest
    %stack(rlp_pos, cur_len, rlp_start, retdest) -> (rlp_pos, rlp_start, cur_len, retdest)
encode_node_branch_prepend_prefix:
    // stack: rlp_pos'', rlp_start, cur_len, retdest
    %prepend_rlp_list_prefix
    // stack: rlp_prefix_start, rlp_len, cur_len, retdest
    %stack (rlp_prefix_start, rlp_len, cur_len, retdest)
        -> (retdest, rlp_prefix_start, rlp_len, cur_len)
    JUMP


// Part of the encode_node_branch function. Encodes the i'th child.
%macro encode_child(i)
    // stack: rlp_pos, rlp_start, node_payload_ptr, encode_value, cur_len, retdest
    PUSH %%after_encode
    DUP6 DUP6 DUP6
    // stack: node_payload_ptr, encode_value, cur_len, %%after_encode, rlp_pos, rlp_start, node_payload_ptr, encode_value, cur_len, retdest
    %add_const($i) %mload_trie_data
    // stack: child_i_ptr, encode_value, cur_len, %%after_encode, rlp_pos, rlp_start, node_payload_ptr, encode_value, cur_len, retdest
    %stack 
        (child_i_ptr, encode_value, cur_len, after_encode, rlp_pos, rlp_start, node_payload_ptr, encode_value, cur_len, retdest) ->
        (child_i_ptr, encode_value, cur_len, after_encode, rlp_pos, rlp_start, node_payload_ptr, encode_value, retdest)
    %jump(encode_or_hash_node)
%%after_encode:
    // stack: result, result_len, cur_len, rlp_pos, rlp_start, node_payload_ptr, encode_value, retdest
    // If result_len != 32, result is raw RLP, with an appropriate RLP prefix already.
    SWAP1 DUP1 %sub_const(32) %jumpi(%%unpack)
    // Otherwise, result is a hash, and we need to add the prefix 0x80 + 32 = 160.
    // stack: result_len, result, cur_len, rlp_pos, rlp_start, node_payload_ptr, encode_value, retdest
    DUP4 // rlp_pos
    PUSH 160
    MSTORE_GENERAL
    SWAP3 %increment SWAP3 // rlp_pos += 1
%%unpack:
    %stack (result_len, result, cur_len, rlp_pos, rlp_start, node_payload_ptr, encode_value, retdest)
        -> (rlp_pos, result, result_len, %%after_unpacking,
            rlp_start, node_payload_ptr, encode_value, cur_len, retdest)
    %jump(mstore_unpacking)
%%after_unpacking:
    // stack: rlp_pos', rlp_start, node_payload_ptr, encode_value, cur_len, retdest
%endmacro

global encode_node_extension:
    // stack: node_type, node_payload_ptr, encode_value, cur_len, retdest
    SWAP3 %add_const(4) SWAP3
    %stack (node_type, node_payload_ptr, encode_value, cur_len)
        -> (node_payload_ptr, encode_value, cur_len, encode_node_extension_after_encode_child, node_payload_ptr)
    %add_const(2) %mload_trie_data
    // stack: child_ptr, encode_value, cur_len, encode_node_extension_after_encode_child, node_payload_ptr, retdest
    %jump(encode_or_hash_node)
encode_node_extension_after_encode_child:
    // stack: result, result_len, cur_len, node_payload_ptr, retdest
    %stack (result, result_len, cur_len, node_payload_ptr) -> (result, result_len, node_payload_ptr, cur_len)
    %alloc_rlp_block
    // stack: rlp_start, result, result_len, node_payload_ptr, cur_len, retdest
    PUSH encode_node_extension_after_hex_prefix // retdest
    PUSH 0 // terminated
    // stack: terminated, encode_node_extension_after_hex_prefix, rlp_start, result, result_len, node_payload_ptr, cur_len, retdest
    DUP6 %increment %mload_trie_data // Load the packed_nibbles field, which is at index 1.
    // stack: packed_nibbles, terminated, encode_node_extension_after_hex_prefix, rlp_start, result, result_len, node_payload_ptr, cur_len, retdest
    DUP7 %mload_trie_data // Load the num_nibbles field, which is at index 0.
    // stack: num_nibbles, packed_nibbles, terminated, encode_node_extension_after_hex_prefix, rlp_start, result, result_len, node_payload_ptr, cur_len, retdest
    DUP5
    // stack: rlp_start, num_nibbles, packed_nibbles, terminated, encode_node_extension_after_hex_prefix, rlp_start, result, result_len, node_payload_ptr, cur_len, retdest
    %jump(hex_prefix_rlp)
encode_node_extension_after_hex_prefix:
    // stack: rlp_pos, rlp_start, result, result_len, node_payload_ptr, cur_len, retdest
    // If result_len != 32, result is raw RLP, with an appropriate RLP prefix already.
    DUP4 %sub_const(32) %jumpi(encode_node_extension_unpack)
    // Otherwise, result is a hash, and we need to add the prefix 0x80 + 32 = 160.
    DUP1 // rlp_pos
    PUSH 160
    MSTORE_GENERAL
    %increment // rlp_pos += 1
encode_node_extension_unpack:
    %stack (rlp_pos, rlp_start, result, result_len, node_payload_ptr, cur_len)
        -> (rlp_pos, result, result_len, encode_node_extension_after_unpacking, rlp_start, cur_len)
    %jump(mstore_unpacking)
encode_node_extension_after_unpacking:
    // stack: rlp_pos, rlp_start, cur_len, retdest
    %prepend_rlp_list_prefix
    %stack (rlp_prefix_start_pos, rlp_len, cur_len, retdest)
        -> (retdest, rlp_prefix_start_pos, rlp_len, cur_len)
    JUMP

global encode_node_leaf:
    // stack: node_type, node_payload_ptr, encode_value, cur_len, retdest
    // `TrieData` holds the node type, the number of nibbles, the nibbles,
    // the pointer to the value and the value.
    // First, we add 4 for the node type, the number of nibbles, the nibbles
    // and the pointer to the value.
    SWAP3 %add_const(4) SWAP3
    POP
    // stack: node_payload_ptr, encode_value, cur_len, retdest
    %alloc_rlp_block
    PUSH encode_node_leaf_after_hex_prefix // retdest
    PUSH 1 // terminated
    // stack: terminated, encode_node_leaf_after_hex_prefix, rlp_start, node_payload_ptr, encode_value, cur_len, retdest
    DUP4 %increment %mload_trie_data // Load the packed_nibbles field, which is at index 1.
    // stack: packed_nibbles, terminated, encode_node_leaf_after_hex_prefix, rlp_start, node_payload_ptr, encode_value, cur_len, retdest
    DUP5 %mload_trie_data // Load the num_nibbles field, which is at index 0.
    // stack: num_nibbles, packed_nibbles, terminated, encode_node_leaf_after_hex_prefix, rlp_start, node_payload_ptr, encode_value, cur_len, retdest
    DUP5
    // stack: rlp_start, num_nibbles, packed_nibbles, terminated, encode_node_leaf_after_hex_prefix, rlp_start, node_payload_ptr, encode_value, cur_len, retdest
    %jump(hex_prefix_rlp)
encode_node_leaf_after_hex_prefix:
    // stack: rlp_pos, rlp_start, node_payload_ptr, encode_value, cur_len, retdest
    SWAP2
    %add_const(2) // The value pointer starts at index 3, after num_nibbles and packed_nibbles.
    // stack: value_ptr_ptr, rlp_start, rlp_pos, encode_value, cur_len, retdest
    %mload_trie_data
    // stack: value_ptr, rlp_start, rlp_pos, encode_value, cur_len, retdest
    %stack (value_ptr, rlp_start, rlp_pos, encode_value, cur_len, retdest)
        -> (encode_value, rlp_pos, value_ptr, cur_len, encode_node_leaf_after_encode_value, rlp_start, retdest)
    JUMP
encode_node_leaf_after_encode_value:
    // stack: rlp_end_pos, cur_len, rlp_start, retdest
    %stack(rlp_end_pos, cur_len, rlp_start, retdest) -> (rlp_end_pos, rlp_start, cur_len, retdest)
    %prepend_rlp_list_prefix
    %stack (rlp_prefix_start_pos, rlp_len, cur_len, retdest)
        -> (retdest, rlp_prefix_start_pos, rlp_len, cur_len)
    JUMP
