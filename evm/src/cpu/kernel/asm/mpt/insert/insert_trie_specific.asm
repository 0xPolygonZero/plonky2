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

// Insert a node in the transaction trie. The payload
// must be pointing to the rlp encoded txn
// Pre stack: key, txn_rlp_ptr, redest
// Post stack: (empty)
global mpt_insert_txn_trie:
    // stack: key=rlp(key), num_nibbles, txn_rlp_ptr, retdest 
    %stack (key, num_nibbles, txn_rlp_ptr)
        -> (num_nibbles, key, txn_rlp_ptr, mpt_insert_txn_trie_save)
    %mload_global_metadata(@GLOBAL_METADATA_TXN_TRIE_ROOT)
    // stack: txn_trie_root_ptr, num_nibbles, key, txn_rlp_ptr, mpt_insert_state_trie_save, retdest
    %jump(mpt_insert)

mpt_insert_txn_trie_save:
    // stack: updated_node_ptr, retdest
    %mstore_global_metadata(@GLOBAL_METADATA_TXN_TRIE_ROOT)
    JUMP

%macro mpt_insert_txn_trie
    %stack (key, txn_rpl_ptr) -> (key, txn_rlp_ptr, %%after)
    %jump(mpt_insert_txn_trie)
%%after:
%endmacro

global mpt_insert_receipt_trie:
    // stack: num_nibbles, scalar, value_ptr, retdest
    %stack (num_nibbles, scalar, value_ptr)
        -> (num_nibbles, scalar, value_ptr, mpt_insert_receipt_trie_save)
    // The key is the scalar, which is an RLP encoding of the transaction number
    // stack: num_nibbles, key, value_ptr, mpt_insert_receipt_trie_save, retdest
    %mload_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_ROOT)
    // stack: receipt_root_ptr, num_nibbles, key, value_ptr, mpt_insert_receipt_trie_save, retdest
    %jump(mpt_insert)
mpt_insert_receipt_trie_save:
    // stack: updated_node_ptr, retdest
    %mstore_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_ROOT)
    JUMP

%macro mpt_insert_receipt_trie
    %stack (num_nibbles, key, value_ptr) -> (num_nibbles, key, value_ptr, %%after)
    %jump(mpt_insert_receipt_trie)
%%after:
%endmacro

// Pre stack: scalar, retdest
// Post stack: rlp_scalar
global scalar_to_rlp:
    // stack: scalar, retdest
    %mload_global_metadata(@GLOBAL_METADATA_RLP_DATA_SIZE)
    // stack: init_addr, scalar, retdest
    SWAP1 DUP2
    %encode_rlp_scalar
    // stack: addr', init_addr, retdest
    // Now our rlp_encoding is in RlpRaw.
    // Set new RlpRaw data size
    DUP1 %mstore_global_metadata(@GLOBAL_METADATA_RLP_DATA_SIZE)
    DUP2 DUP2 SUB // len of the key
    // stack: len, addr', init_addr, retdest
    DUP3
    %mload_packing
    // stack: packed_key, addr', init_addr, retdest
    SWAP2 %pop2
    // stack: key, retdest
    SWAP1
    JUMP

%macro scalar_to_rlp
    %stack (scalar) -> (scalar, %%after)
    %jump(scalar_to_rlp)
%%after:
%endmacro
