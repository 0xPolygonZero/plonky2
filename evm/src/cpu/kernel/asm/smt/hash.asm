%macro smt_hash_state
    %stack (cur_len) -> (cur_len, %%after)
    %jump(smt_hash_state)
%%after:
%endmacro

// Root hash of the state SMT.
global smt_hash_state:
    // stack: cur_len, retdest
    %mload_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT)

// Root hash of SMT stored at `trie_data[ptr]`.
// Pseudocode:
// ```
// hash( HashNode { h } ) = h
// hash( InternalNode { left, right } ) = Poseidon(hash(left) || hash(right) || [0,0,0,0])
// hash( Leaf { rem_key, val_hash } ) = Poseidon(rem_key || val_hash || [1,0,0,0])
// ```
// where `val_hash` is `keccak(nonce || balance || storage_root || code_hash)` for accounts and
// `val` for a storage value.
global smt_hash:
    // stack: ptr, cur_len, retdest
    DUP1
    %mload_trie_data
    // stack: node, node_ptr, cur_len, retdest
    DUP1 %eq_const(@SMT_NODE_HASH) %jumpi(smt_hash_hash)
    DUP1 %eq_const(@SMT_NODE_INTERNAL) %jumpi(smt_hash_internal)
    DUP1 %eq_const(@SMT_NODE_LEAF) %jumpi(smt_hash_leaf)
smt_hash_unknown_node_type:
    PANIC

smt_hash_hash:
    // stack: node, node_ptr, cur_len, retdest
    POP
    // stack: node_ptr, cur_len, retdest
    SWAP1 %add_const(2) SWAP1
    // stack: node_ptr, cur_len, retdest
    %increment
    // stack: node_ptr+1, cur_len, retdest
    %mload_trie_data
    %stack (hash, cur_len, retdest) -> (retdest, hash, cur_len)
    JUMP

smt_hash_internal:
    // stack: node, node_ptr, cur_len, retdest
    POP
    // stack: node_ptr, cur_len, retdest
    SWAP1 %add_const(3) SWAP1
    %increment
    // stack: node_ptr+1, cur_len, retdest
    DUP1
    %mload_trie_data
    %stack (left_child_ptr, node_ptr_plus_1, cur_len, retdest) -> (left_child_ptr, cur_len, smt_hash_internal_after_left, node_ptr_plus_1, retdest)
    %jump(smt_hash)
smt_hash_internal_after_left:
    %stack (left_hash, cur_len, node_ptr_plus_1, retdest) -> (left_hash, node_ptr_plus_1, cur_len, retdest)
    SWAP1 %increment
    // stack: node_ptr+2, left_hash, cur_len, retdest
    %mload_trie_data
    %stack (right_child_ptr, left_hash, cur_len, retdest) -> (right_child_ptr, cur_len, smt_hash_internal_after_right, left_hash, retdest)
    %jump(smt_hash)
smt_hash_internal_after_right:
    %stack (right_hash, cur_len, left_hash) -> (left_hash, right_hash, 0, cur_len)
    POSEIDON
    %stack (hash, cur_len, retdest) -> (retdest, hash, cur_len)
    JUMP

global smt_hash_leaf:
    // stack: node, node_ptr, cur_len, retdest
    POP
    // stack: node_ptr, cur_len, retdest
    SWAP1 %add_const(3) SWAP1
    // stack: node_ptr, cur_len, retdest
    %increment
    // stack: node_ptr+1, cur_len, retdest
    DUP1 %increment
global lalol:
    // stack: node_ptr+2, node_ptr+1, cur_len, retdest
    %mload_trie_data
    // stack: value, node_ptr+1, cur_len, retdest
    SWAP1
    // stack: node_ptr+1, value, cur_len, retdest
    %mload_trie_data
    // stack: rem_key, value, cur_len, retdest
    SWAP1
    // stack: value, rem_key, cur_len, retdest
    %split_value
    %stack (v0, v1) -> (v0, v1, 0)
    POSEIDON
    %stack (value_hash, rem_key) -> (rem_key, value_hash, 1)
    POSEIDON
    %stack (hash, cur_len, retdest) -> (retdest, hash, cur_len)
    JUMP


// value = sum_{0<=i<8} (a_i << (i*32))
// return (sum_{0<=i<4} (a_i << (i*64)), sum_{4<=i<8} (a_i << ((i-4)*64)))
%macro split_value
    // stack: value
    DUP1 %and_const(0xffffffff)
    // stack: a_0, value
    DUP2 %shr_const(32) %and_const(0xffffffff)
    // stack: a_1, a_0, value
    %shl_const(64) ADD
    // stack: a_0 + a_1<<64, value
    DUP2 %shr_const(64) %and_const(0xffffffff)
    // stack: a_2, a_0 + a_1<<64, value
    %shl_const(128) ADD
    // stack: a_0 + a_1<<64 + a_2<<128, value
    DUP2 %shr_const(96) %and_const(0xffffffff)
    // stack: a_3, a_0 + a_1<<64 + a_2<<128, value
    %shl_const(192) ADD
    // stack: a_0 + a_1<<64 + a_2<<128 + a_3<<196, value
    DUP2 %shr_const(128) %and_const(0xffffffff)
    // stack: a_4, a_0 + a_1<<64 + a_2<<128 + a_3<<196, value
    DUP3 %shr_const(160) %and_const(0xffffffff)
    // stack: a_5, a_4, a_0 + a_1<<64 + a_2<<128 + a_3<<196, value
    %shl_const(64) ADD
    // stack: a_4 + a_5<<64, a_0 + a_1<<64 + a_2<<128 + a_3<<196, value
    DUP3 %shr_const(192) %and_const(0xffffffff)
    // stack: a_6, a_4 + a_5<<64, a_0 + a_1<<64 + a_2<<128 + a_3<<196, value
    %shl_const(128) ADD
    // stack: a_4 + a_5<<64 + a_6<<128, a_0 + a_1<<64 + a_2<<128 + a_3<<196, value
    DUP3 %shr_const(224) %and_const(0xffffffff)
    // stack: a_7, a_4 + a_5<<64 + a_6<<128, a_0 + a_1<<64 + a_2<<128 + a_3<<196, value
    %shl_const(192) ADD
    // stack: a_4 + a_5<<64 + a_6<<128 + a_7<<192, a_0 + a_1<<64 + a_2<<128 + a_3<<196, value
    SWAP2 POP
    // stack: a_0 + a_1<<64 + a_2<<128 + a_3<<196, a_4 + a_5<<64 + a_6<<128 + a_7<<192
%endmacro