// Hashing logic specific to a particular trie.

global mpt_hash_state_trie:
    // stack: retdest
    PUSH encode_account
    %mload_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT)
    // stack: node_ptr, encode_account, retdest
    %jump(mpt_hash)

%macro mpt_hash_state_trie
    PUSH %%after
    %jump(mpt_hash_state_trie)
%%after:
%endmacro

global mpt_hash_storage_trie:
    // stack: node_ptr, retdest
    %stack (node_ptr) -> (node_ptr, encode_storage_value)
    %jump(mpt_hash)

%macro mpt_hash_storage_trie
    %stack (node_ptr) -> (node_ptr, %%after)
    %jump(mpt_hash_storage_trie)
%%after:
%endmacro

global mpt_hash_txn_trie:
    // stack: retdest
    PUSH encode_txn
    %mload_global_metadata(@GLOBAL_METADATA_TXN_TRIE_ROOT)
    // stack: node_ptr, encode_txn, retdest
    %jump(mpt_hash)

%macro mpt_hash_txn_trie
    PUSH %%after
    %jump(mpt_hash_txn_trie)
%%after:
%endmacro

global mpt_hash_receipt_trie:
    // stack: retdest
    PUSH encode_receipt
    %mload_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_ROOT)
    // stack: node_ptr, encode_receipt, retdest
    %jump(mpt_hash)

%macro mpt_hash_receipt_trie
    PUSH %%after
    %jump(mpt_hash_receipt_trie)
%%after:
%endmacro

global encode_account:
    // stack: rlp_pos, value_ptr, retdest
    // First, we compute the length of the RLP data we're about to write.
    // The nonce and balance fields are variable-length, so we need to load them
    // to determine their contribution, while the other two fields are fixed
    // 32-bytes integers.
    DUP2 %mload_trie_data // nonce = value[0]
    %rlp_scalar_len
    // stack: nonce_rlp_len, rlp_pos, value_ptr, retdest
    DUP3 %increment %mload_trie_data // balance = value[1]
    %rlp_scalar_len
    // stack: balance_rlp_len, nonce_rlp_len, rlp_pos, value_ptr, retdest
    PUSH 66 // storage_root and code_hash fields each take 1 + 32 bytes
    ADD ADD
    // stack: payload_len, rlp_pos, value_ptr, retdest
    SWAP1
    // stack: rlp_pos, payload_len, value_ptr, retdest
    DUP2 %rlp_list_len
    // stack: list_len, rlp_pos, payload_len, value_ptr, retdest
    SWAP1
    // stack: rlp_pos, list_len, payload_len, value_ptr, retdest
    %encode_rlp_multi_byte_string_prefix
    // stack: rlp_pos_2, payload_len, value_ptr, retdest
    %encode_rlp_list_prefix
    // stack: rlp_pos_3, value_ptr, retdest
    DUP2 %mload_trie_data // nonce = value[0]
    // stack: nonce, rlp_pos_3, value_ptr, retdest
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos_4, value_ptr, retdest
    DUP2 %increment %mload_trie_data // balance = value[1]
    // stack: balance, rlp_pos_4, value_ptr, retdest
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos_5, value_ptr, retdest
    DUP2 %add_const(2) %mload_trie_data // storage_root_ptr = value[2]
    // stack: storage_root_ptr, rlp_pos_5, value_ptr, retdest
    %mpt_hash_storage_trie
    // stack: storage_root_digest, rlp_pos_5, value_ptr, retdest
    SWAP1 %encode_rlp_256
    // stack: rlp_pos_6, value_ptr, retdest
    SWAP1 %add_const(3) %mload_trie_data // code_hash = value[3]
    // stack: code_hash, rlp_pos_6, retdest
    SWAP1 %encode_rlp_256
    // stack: rlp_pos_7, retdest
    SWAP1
    JUMP

global encode_txn:
    PANIC // TODO

global encode_receipt:
    PANIC // TODO

global encode_storage_value:
    // stack: rlp_pos, value_ptr, retdest
    SWAP1 %mload_trie_data SWAP1
    // stack: rlp_pos, value, retdest
    // The YP says storage trie is a map "... to the RLP-encoded 256-bit integer values"
    // which seems to imply that this should be %encode_rlp_256. But %encode_rlp_scalar
    // causes the tests to pass, so it seems storage values should be treated as variable-
    // length after all.
    %doubly_encode_rlp_scalar
    // stack: rlp_pos', retdest
    SWAP1
    JUMP
