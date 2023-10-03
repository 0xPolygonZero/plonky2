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
    // stack: rlp_pos, value_ptr, retdest
    
    // Load the txn_rlp_len which is at the beginnig of value_ptr
    DUP2 %mload_trie_data
    // stack: txn_rlp_len, rlp_pos, value_ptr, retdest
    SWAP2 %increment
    // stack: txn_rlp_ptr=value_ptr+1, rlp_pos, txn_rlp_len, retdest

    %stack (txn_rlp_ptr, rlp_pos, txn_rlp_len) -> (rlp_pos, txn_rlp_len, txn_rlp_len, txn_rlp_ptr)
    // Encode the txn rlp prefix
    // stack: rlp_pos, txn_rlp_len, txn_rlp_len, txn_rlp_ptr, retdest
    %encode_rlp_multi_byte_string_prefix
    // copy txn_rlp to the new block
    // stack: rlp_pos, txn_rlp_len, txn_rlp_ptr, retdest
    %stack (rlp_pos, txn_rlp_len, txn_rlp_ptr) -> (
        0, @SEGMENT_RLP_RAW, rlp_pos, // dest addr
        0, @SEGMENT_TRIE_DATA, txn_rlp_ptr, // src addr. Kernel has context 0
        txn_rlp_len, // mcpy len
        txn_rlp_len, rlp_pos)
    %memcpy
    ADD
    // stack new_rlp_pos, retdest
    SWAP1
    JUMP

// We assume a receipt in memory is stored as:
// [payload_len, status, cum_gas_used, bloom, logs_payload_len, num_logs, [logs]].
// A log is [payload_len, address, num_topics, [topics], data_len, [data]].
global encode_receipt:
    // stack: rlp_pos, value_ptr, retdest
    // There is a double encoding! What we compute is:
    // either RLP(RLP(receipt)) for Legacy transactions or RLP(txn_type||RLP(receipt)) for transactions of type 1 or 2.
    // First encode the wrapper prefix.
    DUP2 %mload_trie_data
    // stack: first_value, rlp_pos, value_ptr, retdest
    // The first value is either the transaction type or the payload length.
    // Since the receipt contains at least the 256-bytes long bloom filter, payload_len > 3.
    DUP1 %lt_const(3) %jumpi(encode_nonzero_receipt_type)
    // If we are here, then the first byte is the payload length.
    %rlp_list_len
    // stack: rlp_receipt_len, rlp_pos, value_ptr, retdest
    SWAP1 %encode_rlp_multi_byte_string_prefix
    // stack: rlp_pos, value_ptr, retdest

encode_receipt_after_type:
    // stack: rlp_pos, payload_len_ptr, retdest
    // Then encode the receipt prefix.
    // `payload_ptr` is either `value_ptr` or `value_ptr+1`, depending on the transaction type.
    DUP2 %mload_trie_data
    // stack: payload_len, rlp_pos, payload_len_ptr, retdest
    SWAP1 %encode_rlp_list_prefix 
    // stack: rlp_pos, payload_len_ptr, retdest
    // Encode status.
    DUP2 %increment %mload_trie_data
    // stack: status, rlp_pos, payload_len_ptr, retdest
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos, payload_len_ptr, retdest
    // Encode cum_gas_used.
    DUP2 %add_const(2) %mload_trie_data
    // stack: cum_gas_used, rlp_pos, payload_len_ptr, retdest
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos, payload_len_ptr, retdest
    // Encode bloom.
    PUSH 256 // Bloom length.
    DUP3 %add_const(3) PUSH @SEGMENT_TRIE_DATA PUSH 0 // MPT src address.
    DUP5
    // stack: rlp_pos, SRC, 256, rlp_pos, payload_len_ptr, retdest
    %encode_rlp_string
    // stack: rlp_pos, old_rlp_pos, payload_len_ptr, retdest
    SWAP1 POP
    // stack: rlp_pos, payload_len_ptr, retdest
    // Encode logs prefix.
    DUP2 %add_const(259) %mload_trie_data
    // stack: logs_payload_len, rlp_pos, payload_len_ptr, retdest
    SWAP1 %encode_rlp_list_prefix
    // stack: rlp_pos, payload_len_ptr, retdest
    DUP2 %add_const(261)
    // stack: logs_ptr, rlp_pos, payload_len_ptr, retdest
    DUP3 %add_const(260) %mload_trie_data
    // stack: num_logs, logs_ptr, rlp_pos, payload_len_ptr, retdest
    PUSH 0

encode_receipt_logs_loop:
    // stack: i, num_logs, current_log_ptr, rlp_pos, payload_len_ptr, retdest
    DUP2 DUP2 EQ
    // stack: i == num_logs, i, num_logs, current_log_ptr, rlp_pos, payload_len_ptr, retdest
    %jumpi(encode_receipt_end)
    // stack: i, num_logs, current_log_ptr, rlp_pos, payload_len_ptr, retdest
    DUP3 DUP5
    // stack: rlp_pos, current_log_ptr, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    // Encode log prefix.
    DUP2 %mload_trie_data
    // stack: payload_len, rlp_pos, current_log_ptr, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    SWAP1 %encode_rlp_list_prefix
    // stack: rlp_pos, current_log_ptr, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    // Encode address.
    DUP2 %increment %mload_trie_data
    // stack: address, rlp_pos, current_log_ptr, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    SWAP1 %encode_rlp_160
    // stack: rlp_pos, current_log_ptr, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    DUP2 %add_const(2) %mload_trie_data
    // stack: num_topics, rlp_pos, current_log_ptr, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    // Encode topics prefix.
    DUP1 %mul_const(33)
    // stack: topics_payload_len, num_topics, rlp_pos, current_log_ptr, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    DUP3 %encode_rlp_list_prefix
    // stack: new_rlp_pos, num_topics, rlp_pos, current_log_ptr, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    SWAP2 POP
    // stack: num_topics, rlp_pos, current_log_ptr, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    SWAP2 %add_const(3)
    // stack: topics_ptr, rlp_pos, num_topics, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    PUSH 0

encode_receipt_topics_loop:
    // stack: j, topics_ptr, rlp_pos, num_topics, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    DUP4 DUP2 EQ
    // stack: j == num_topics, j, topics_ptr, rlp_pos, num_topics, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    %jumpi(encode_receipt_topics_end)
    // stack: j, topics_ptr, rlp_pos, num_topics, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    DUP2 DUP2 ADD
    %mload_trie_data
    // stack: current_topic, j, topics_ptr, rlp_pos, num_topics, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    DUP4
    // stack: rlp_pos, current_topic, j, topics_ptr, rlp_pos, num_topics, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    %encode_rlp_256
    // stack: new_rlp_pos, j, topics_ptr, rlp_pos, num_topics, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    SWAP3 POP
    // stack: j, topics_ptr, new_rlp_pos, num_topics, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    %increment
    %jump(encode_receipt_topics_loop)

encode_receipt_topics_end:
    // stack: num_topics, topics_ptr, rlp_pos, num_topics, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    ADD
    // stack: data_len_ptr, rlp_pos, num_topics, i, num_logs, current_log_ptr, old_rlp_pos, payload_len_ptr, retdest
    SWAP5 POP
    // stack: rlp_pos, num_topics, i, num_logs, data_len_ptr, old_rlp_pos, payload_len_ptr, retdest
    SWAP5 POP
    // stack: num_topics, i, num_logs, data_len_ptr, rlp_pos, payload_len_ptr, retdest
    POP
    // stack: i, num_logs, data_len_ptr, rlp_pos, payload_len_ptr, retdest
    // Encode data prefix.
    DUP3 %mload_trie_data
    // stack: data_len, i, num_logs, data_len_ptr, rlp_pos, payload_len_ptr, retdest
    DUP4 %increment DUP2 ADD
    // stack: next_log_ptr, data_len, i, num_logs, data_len_ptr, rlp_pos, payload_len_ptr, retdest
    SWAP4 %increment
    // stack: data_ptr, data_len, i, num_logs, next_log_ptr, rlp_pos, payload_len_ptr, retdest
    PUSH @SEGMENT_TRIE_DATA PUSH 0
    // stack: SRC, data_len, i, num_logs, next_log_ptr, rlp_pos, payload_len_ptr, retdest
    DUP8
    // stack: rlp_pos, SRC, data_len, i, num_logs, next_log_ptr, rlp_pos, payload_len_ptr, retdest
    %encode_rlp_string
    // stack: new_rlp_pos, i, num_logs, next_log_ptr, rlp_pos, payload_len_ptr, retdest
    SWAP4 POP
    // stack: i, num_logs, next_log_ptr, new_rlp_pos, payload_len_ptr, retdest
    %increment
    %jump(encode_receipt_logs_loop)

encode_receipt_end:
    // stack: num_logs, num_logs, current_log_ptr, rlp_pos, payload_len_ptr, retdest
    %pop3
    // stack: rlp_pos, payload_len_ptr, retdest
    SWAP1 POP
    // stack: rlp_pos, retdest
    SWAP1
    JUMP

encode_nonzero_receipt_type:
    // stack: txn_type, rlp_pos, value_ptr, retdest
    DUP3 %increment %mload_trie_data
    // stack: payload_len, txn_type, rlp_pos, value_ptr, retdest
    // The transaction type is encoded in 1 byte
    %increment %rlp_list_len
    // stack: rlp_receipt_len, txn_type, rlp_pos, value_ptr, retdest
    DUP3 %encode_rlp_multi_byte_string_prefix
    // stack: rlp_pos, txn_type, old_rlp_pos, value_ptr, retdest
    DUP2 DUP2
    %mstore_rlp
    %increment
    // stack: rlp_pos, txn_type, old_rlp_pos, value_ptr, retdest
    %stack (rlp_pos, txn_type, old_rlp_pos, value_ptr, retdest) -> (rlp_pos, value_ptr, retdest)
    // We replace `value_ptr` with `paylaod_len_ptr` so we can encode the rest of the data more easily
    SWAP1 %increment SWAP1
    // stack: rlp_pos, payload_len_ptr, retdest
    %jump(encode_receipt_after_type)

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

