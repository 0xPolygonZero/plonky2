// Insertion logic specific to a particular trie.

// Mutate the state trie, inserting the given key-value pair.
// Pre stack: key, value_ptr, retdest
// Post stack: (empty)
// TODO: Have this take an address and do %mpt_insert_state_trie? To match mpt_read_state_trie.
global mpt_insert_state_trie:
    // stack: key, value_ptr, retdest
    %stack (key, value_ptr)
        -> (key, value_ptr, mpt_insert_state_trie_save)
    PUSH 64 // num_nibbles
    %mload_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT)
    // stack: state_root_ptr, num_nibbles, key, value_ptr, mpt_insert_state_trie_save, retdest
    %jump(mpt_insert)
mpt_insert_state_trie_save:
    // stack: updated_node_ptr, retdest
    %mstore_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT)
    JUMP

%macro mpt_insert_state_trie
    %stack (key, value_ptr) -> (key, value_ptr, %%after)
    %jump(mpt_insert_state_trie)
%%after:
%endmacro

global mpt_insert_receipt_trie:
    // stack: scalar, value_ptr, retdest
    %stack (scalar, value_ptr)
        -> (scalar, value_ptr, mpt_insert_receipt_trie_save)
    // The key is the RLP encoding of scalar.
    %scalar_to_rlp
    // stack: key, value_ptr, mpt_insert_receipt_trie_save, retdest
    DUP1
    %num_bytes %mul_const(2)
    // stack: num_nibbles, key, value_ptr, mpt_insert_receipt_trie_save, retdest
    %mload_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_ROOT)
    // stack: receipt_root_ptr, num_nibbles, key, value_ptr, mpt_insert_receipt_trie_save, retdest
    %jump(mpt_insert)
mpt_insert_receipt_trie_save:
    // stack: updated_node_ptr, retdest
    %mstore_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_ROOT)
    JUMP

%macro mpt_insert_receipt_trie
    %stack (key, value_ptr) -> (key, value_ptr, %%after)
    %jump(mpt_insert_receipt_trie)
%%after:
%endmacro

// Pre stack: scalar, retdest
// Post stack: rlp_scalar
// We will make use of %encode_rlp_scalar, which clobbers RlpRaw.
// We're not hashing tries yet, so it's not an issue.
global scalar_to_rlp:
    // stack: scalar, retdest
    PUSH 0
    // stack: pos, scalar, retdest
    %encode_rlp_scalar
    // stack: pos', retdest
    // Now our rlp_encoding is in RlpRaw in the first pos' cells.
    DUP1 // len of the key
    PUSH 0 PUSH @SEGMENT_RLP_RAW PUSH 0 // address where we get the key from
    %mload_packing
    // stack: packed_key, pos', retdest
    SWAP1 POP
    // stack: key, retdest
    SWAP1
    JUMP

%macro scalar_to_rlp
    %stack (scalar) -> (scalar, %%after)
    %jump(scalar_to_rlp)
%%after:
%endmacro
