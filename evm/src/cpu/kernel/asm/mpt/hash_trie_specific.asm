// Hashing logic specific to a particular trie.

global mpt_hash_state_trie:
    // stack: retdest
    %mload_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT)
    // stack: node_ptr, retdest
    %mpt_hash(encode_account)

%macro mpt_hash_state_trie
    PUSH %%after
    %jump(mpt_hash_state_trie)
%%after:
%endmacro

global mpt_hash_txn_trie:
    // stack: retdest
    %mload_global_metadata(@GLOBAL_METADATA_TXN_TRIE_ROOT)
    // stack: node_ptr, retdest
    %mpt_hash(encode_txn)

%macro mpt_hash_txn_trie
    PUSH %%after
    %jump(mpt_hash_txn_trie)
%%after:
%endmacro

global mpt_hash_receipt_trie:
    // stack: retdest
    %mload_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_ROOT)
    // stack: node_ptr, retdest
    %mpt_hash(encode_receipt)

%macro mpt_hash_receipt_trie
    PUSH %%after
    %jump(mpt_hash_receipt_trie)
%%after:
%endmacro

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

encode_txn:
    PANIC // TODO

encode_receipt:
    PANIC // TODO
