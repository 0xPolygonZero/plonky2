// Pre-stack: status, leftover_gas, prev_cum_gas, txn_nb, num_nibbles, retdest
// Post stack: new_cum_gas, txn_nb
// A receipt is stored in MPT_TRIE_DATA as:
// [payload_len, status, cum_gas_used, bloom, logs_payload_len, num_logs, [logs]]
//
// In this function, we:
// - compute cum_gas, 
// - check if the transaction failed and set number of logs to 0 if it is the case, 
// - compute the bloom filter,
// - write the receipt in MPT_TRIE_DATA ,
// - insert a new node in receipt_trie,
// - set the bloom filter back to 0
global process_receipt:    
    // stack: status, leftover_gas, prev_cum_gas, txn_nb, num_nibbles, retdest
    DUP2 DUP4
    // stack: prev_cum_gas, leftover_gas, status, leftover_gas, prev_cum_gas, txn_nb, num_nibbles, retdest
    %compute_cumulative_gas
    // stack: new_cum_gas, status, leftover_gas, prev_cum_gas, txn_nb, num_nibbles, retdest
    SWAP3 POP
    // stack: status, leftover_gas, new_cum_gas, txn_nb, num_nibbles, retdest
    SWAP1 POP
    // stack: status, new_cum_gas, txn_nb, num_nibbles, retdest
    // Now, we need to check whether the transaction has failed.
    DUP1 ISZERO %jumpi(failed_receipt)

process_receipt_after_status:
    // stack: status, new_cum_gas, txn_nb, num_nibbles, retdest
    PUSH process_receipt_after_bloom
    %jump(logs_bloom)

process_receipt_after_bloom:
    // stack: status, new_cum_gas, txn_nb, num_nibbles, retdest
    DUP2 DUP4
    // stack: txn_nb, new_cum_gas, status, new_cum_gas, txn_nb, num_nibbles, retdest
    SWAP2
    // stack: status, new_cum_gas, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest

    // Compute the total RLP payload length of the receipt.
    PUSH 1 // status is always 1 byte.
    // stack: payload_len, status, new_cum_gas, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    DUP3
    %rlp_scalar_len // cum_gas is a simple scalar.
    ADD
    // stack: payload_len, status, new_cum_gas, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    // Next is the bloom_filter, which is a 256-byte array. Its RLP encoding is 
    // 1 + 2 + 256 bytes.
    %add_const(259)
    // stack: payload_len, status, new_cum_gas, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    // Last is the logs.
    %mload_global_metadata(@GLOBAL_METADATA_LOGS_PAYLOAD_LEN)
    %rlp_list_len
    ADD
    // stack: payload_len, status, new_cum_gas, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    // Now we can write the receipt in MPT_TRIE_DATA.
    %get_trie_data_size
    // stack: receipt_ptr, payload_len, status, new_cum_gas, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    // Write transaction type if necessary. RLP_RAW contains, at index 0, the current transaction type.
    PUSH @SEGMENT_RLP_RAW // ctx == virt == 0
    MLOAD_GENERAL
    // stack: first_txn_byte, receipt_ptr, payload_len, status, new_cum_gas, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    DUP1 %eq_const(1) %jumpi(receipt_nonzero_type)
    DUP1 %eq_const(2) %jumpi(receipt_nonzero_type)
    // If we are here, we are dealing with a legacy transaction, and we do not need to write the type.
    POP

process_receipt_after_type:
    // stack: receipt_ptr, payload_len, status, new_cum_gas, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    // Write payload_len.
    SWAP1
    %append_to_trie_data
    // stack: receipt_ptr, status, new_cum_gas, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    // Write status.
    SWAP1
    %append_to_trie_data
    // stack: receipt_ptr, new_cum_gas, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    // Write cum_gas_used.
    SWAP1
    %append_to_trie_data
    // stack: receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    // Write Bloom filter.
    PUSH 256 // Bloom length.
    PUSH @SEGMENT_TXN_BLOOM // ctx == virt == 0
    // stack: bloom_addr, 256, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    %get_trie_data_size
    PUSH @SEGMENT_TRIE_DATA ADD // MPT dest address.
    // stack: DST, SRC, 256, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    %memcpy_bytes
    // stack: receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    // Update trie data size.
    %get_trie_data_size
    %add_const(256)
    %set_trie_data_size

    // Now we write logs.
    // stack: receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    // We start with the logs payload length.
    %mload_global_metadata(@GLOBAL_METADATA_LOGS_PAYLOAD_LEN)
    %append_to_trie_data
    // stack: receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    %mload_global_metadata(@GLOBAL_METADATA_LOGS_LEN)
    // Then the number of logs.
    // stack: num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    DUP1 %append_to_trie_data
    PUSH 0

// Each log is written in MPT_TRIE_DATA as:
// [payload_len, address, num_topics, [topics], data_len, [data]].
process_receipt_logs_loop:
    // stack: i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    DUP2 DUP2
    EQ
    // stack: i == num_logs, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    %jumpi(process_receipt_after_write)
    // stack: i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    DUP1
    %mload_kernel(@SEGMENT_LOGS)
    // stack: log_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    // Write payload_len.
    DUP1
    %mload_kernel(@SEGMENT_LOGS_DATA)
    %append_to_trie_data
    // stack: log_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    // Write address.
    %increment
    // stack: addr_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    DUP1
    %mload_kernel(@SEGMENT_LOGS_DATA)
    %append_to_trie_data
    // stack: addr_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    //Write num_topics.
    %increment
    // stack: num_topics_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    DUP1
    %mload_kernel(@SEGMENT_LOGS_DATA)
    // stack: num_topics, num_topics_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    DUP1
    %append_to_trie_data
    // stack: num_topics, num_topics_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    SWAP1 %increment SWAP1
    // stack: num_topics, topics_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    PUSH 0

process_receipt_topics_loop:
    // stack: j, num_topics, topics_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    DUP2 DUP2
    EQ
    // stack: j == num_topics, j, num_topics, topics_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    %jumpi(process_receipt_topics_end)
    // stack: j, num_topics, topics_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    // Write j-th topic.
    DUP3 DUP2
    ADD
    // stack: cur_topic_ptr, j, num_topics, topics_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    %mload_kernel(@SEGMENT_LOGS_DATA)
    %append_to_trie_data
    // stack: j, num_topics, topics_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    %increment
    %jump(process_receipt_topics_loop)

process_receipt_topics_end:
    // stack: num_topics, num_topics, topics_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    POP
    ADD
    // stack: data_len_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    // Write data_len
    DUP1
    %mload_kernel(@SEGMENT_LOGS_DATA)
    // stack: data_len, data_len_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    DUP1
    %append_to_trie_data
    // stack: data_len, data_len_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    SWAP1 %increment SWAP1
    // stack: data_len, data_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    PUSH 0

process_receipt_data_loop:
    // stack: j, data_len, data_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    DUP2 DUP2
    EQ
    // stack: j == data_len, j, data_len, data_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    %jumpi(process_receipt_data_end)
    // stack: j, data_len, data_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    // Write j-th data byte.
    DUP3 DUP2
    ADD
    // stack: cur_data_ptr, j, data_len, data_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    %mload_kernel(@SEGMENT_LOGS_DATA)
    %append_to_trie_data
    // stack: j, data_len, data_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    %increment
    %jump(process_receipt_data_loop)

process_receipt_data_end:
    // stack: data_len, data_len, data_ptr, i, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    %pop3
    %increment
    %jump(process_receipt_logs_loop)

process_receipt_after_write:
    // stack: num_logs, num_logs, receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    %pop2
    // stack: receipt_ptr, txn_nb, new_cum_gas, txn_nb, num_nibbles, retdest
    SWAP1
    // stack: txn_nb, receipt_ptr, new_cum_gas, txn_nb, num_nibbles, retdest
    DUP5
    %mpt_insert_receipt_trie
    // stack: new_cum_gas, txn_nb, num_nibbles, retdest
    // Now, we set the Bloom filter back to 0. We proceed by chunks of 32 bytes.
    PUSH @SEGMENT_TXN_BLOOM // ctx == offset == 0
    %rep 8
        // stack: addr, new_cum_gas, txn_nb, num_nibbles, retdest
        PUSH 0 // we will fill the memory segment with zeroes
        DUP2
        // stack: addr, 0, addr, new_cum_gas, txn_nb, num_nibbles, retdest
        MSTORE_32BYTES_32
        // stack: new_addr, addr, new_cum_gas, txn_nb, num_nibbles, retdest
        SWAP1 POP
    %endrep
    POP
    // stack: new_cum_gas, txn_nb, num_nibbles, retdest
    %stack (new_cum_gas, txn_nb, num_nibbles, retdest) -> (retdest, new_cum_gas)
    JUMP
    
receipt_nonzero_type:
    // stack: txn_type, receipt_ptr, payload_len, status, new_cum_gas, txn_nb, new_cum_gas, txn_nb, retdest
    %append_to_trie_data
    %jump(process_receipt_after_type)

failed_receipt:
    // stack: status, new_cum_gas, num_nibbles, txn_nb
    // It is the receipt of a failed transaction, so set num_logs to 0. This will also lead to Bloom filter = 0.
    PUSH 0
    %mstore_global_metadata(@GLOBAL_METADATA_LOGS_LEN)
    PUSH 0 %mstore_global_metadata(@GLOBAL_METADATA_LOGS_PAYLOAD_LEN)
    // stack: status, new_cum_gas, num_nibbles, txn_nb
    %jump(process_receipt_after_status)

%macro process_receipt
    // stack: success, leftover_gas, cur_cum_gas, txn_nb, num_nibbles
    %stack (success, leftover_gas, cur_cum_gas, txn_nb, num_nibbles) -> (success, leftover_gas, cur_cum_gas, txn_nb, num_nibbles, %%after)
    %jump(process_receipt)
%%after:
%endmacro

%macro compute_cumulative_gas
    // stack: cur_cum_gas, leftover_gas
    DUP2
    // stack: leftover_gas, prev_cum_gas, leftover_gas
    %mload_txn_field(@TXN_FIELD_GAS_LIMIT)
    // stack: gas_limit, leftover_gas, prev_cum_gas, leftover_gas
    DUP2 DUP2 LT %jumpi(panic)
    // stack: gas_limit, leftover_gas, prev_cum_gas, leftover_gas
    SUB
    // stack: used_txn_gas, prev_cum_gas, leftover_gas
    ADD SWAP1 POP
    // stack: new_cum_gas
%endmacro
