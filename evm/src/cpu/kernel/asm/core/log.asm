global sys_log0:
    %check_static
    // stack: kexit_info, offset, size
    DUP3 ISZERO %jumpi(log0_after_mem_gas)
    DUP3 DUP3
    %add_or_fault
    // stack: offset+size, kexit_info, offset, size
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes
log0_after_mem_gas:
    // stack: kexit_info, offset, size
    DUP3 %mul_const(@GAS_LOGDATA) %add_const(@GAS_LOG)
    // stack: gas, kexit_info, offset, size
    %charge_gas
    %address
    PUSH 0
    %stack (zero, address, kexit_info, offset, size) -> (address, zero, size, offset, finish_sys_log, kexit_info)
    %jump(log_n_entry)

global sys_log1:
    %check_static
    // stack: kexit_info, offset, size, topic
    DUP3 ISZERO %jumpi(log1_after_mem_gas)
    DUP3 DUP3
    %add_or_fault
    // stack: offset+size, kexit_info, offset, size, topic
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes
log1_after_mem_gas:
    // stack: kexit_info, offset, size, topic
    DUP3 %mul_const(@GAS_LOGDATA) %add_const(@GAS_LOG) %add_const(@GAS_LOGTOPIC)
    // stack: gas, kexit_info, offset, size, topic
    %charge_gas
    %address
    PUSH 1
    %stack (one, address, kexit_info, offset, size, topic) -> (address, one, topic, size, offset, finish_sys_log, kexit_info)
    %jump(log_n_entry)

global sys_log2:
    %check_static
    // stack: kexit_info, offset, size, topic1, topic2
    DUP3 ISZERO %jumpi(log2_after_mem_gas)
    DUP3 DUP3
    %add_or_fault
    // stack: offset+size, kexit_info, offset, size, topic1, topic2
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes
log2_after_mem_gas:
    // stack: kexit_info, offset, size, topic1, topic2
    DUP3 %mul_const(@GAS_LOGDATA) %add_const(@GAS_LOG) %add_const(@GAS_LOGTOPIC) %add_const(@GAS_LOGTOPIC)
    // stack: gas, kexit_info, offset, size, topic1, topic2
    %charge_gas
    %address
    PUSH 2
    %stack (two, address, kexit_info, offset, size, topic1, topic2) -> (address, two, topic1, topic2, size, offset, finish_sys_log, kexit_info)
    %jump(log_n_entry)

global sys_log3:
    %check_static
    // stack: kexit_info, offset, size, topic1, topic2, topic3
    DUP3 ISZERO %jumpi(log3_after_mem_gas)
    DUP3 DUP3
    %add_or_fault
    // stack: offset+size, kexit_info, offset, size, topic1, topic2, topic3
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes
log3_after_mem_gas:
    // stack: kexit_info, offset, size, topic1, topic2, topic3
    DUP3 %mul_const(@GAS_LOGDATA) %add_const(@GAS_LOG) %add_const(@GAS_LOGTOPIC) %add_const(@GAS_LOGTOPIC) %add_const(@GAS_LOGTOPIC)
    // stack: gas, kexit_info, offset, size, topic1, topic2, topic3
    %charge_gas
    %address
    PUSH 3
    %stack (three, address, kexit_info, offset, size, topic1, topic2, topic3) -> (address, three, topic1, topic2, topic3, size, offset, finish_sys_log, kexit_info)
    %jump(log_n_entry)

global sys_log4:
    %check_static
    // stack: kexit_info, offset, size, topic1, topic2, topic3, topic4
    DUP3 ISZERO %jumpi(log4_after_mem_gas)
    DUP3 DUP3
    %add_or_fault
    // stack: offset+size, kexit_info, offset, size, topic1, topic2, topic3, topic4
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes
log4_after_mem_gas:
    // stack: kexit_info, offset, size, topic1, topic2, topic3, topic4
    DUP3 %mul_const(@GAS_LOGDATA) %add_const(@GAS_LOG) %add_const(@GAS_LOGTOPIC) %add_const(@GAS_LOGTOPIC) %add_const(@GAS_LOGTOPIC) %add_const(@GAS_LOGTOPIC)
    // stack: gas, kexit_info, offset, size, topic1, topic2, topic3, topic4
    %charge_gas
    %address
    PUSH 4
    %stack (four, address, kexit_info, offset, size, topic1, topic2, topic3, topic4) -> (address, four, topic1, topic2, topic3, topic4, size, offset, finish_sys_log, kexit_info)
    %jump(log_n_entry)

finish_sys_log:
    // stack: kexit_info
    EXIT_KERNEL

global log_n_entry:
    // stack: address, num_topics, topics, data_len, data_offset, retdest
    %mload_global_metadata(@GLOBAL_METADATA_LOGS_LEN)
    %mload_global_metadata(@GLOBAL_METADATA_LOGS_DATA_LEN)
    // stack: log_ptr, logs_len, address, num_topics, topics, data_len, data_offset, retdest
    DUP1 DUP3
    // stack: log_ptr, logs_len, log_ptr, logs_len, address, num_topics, topics, data_len, data_offset, retdest
    %mstore_kernel(@SEGMENT_LOGS)
    // stack: log_ptr, logs_len, address, num_topics, topics, data_len, data_offset, retdest
    SWAP1 %increment
    %mstore_global_metadata(@GLOBAL_METADATA_LOGS_LEN)
    // stack: log_ptr, address, num_topics, topics, data_len, data_offset, retdest
    %increment
    // stack: addr_ptr, address, num_topics, topics, data_len, data_offset, retdest
    // Store the address.
    DUP2 DUP2
    %mstore_kernel(@SEGMENT_LOGS_DATA)
    %increment
    // stack: num_topics_ptr, address, num_topics, topics, data_len, data_offset, retdest
    SWAP1 POP
    // stack: num_topics_ptr, num_topics, topics, data_len, data_offset, retdest
    // Store num_topics.
    DUP2 DUP2
    %mstore_kernel(@SEGMENT_LOGS_DATA)
    %increment
    // stack: topics_ptr, num_topics, topics, data_len, data_offset, retdest
    DUP2
    // stack: num_topics, topics_ptr, num_topics, topics, data_len, data_offset, retdest
    ISZERO
    %jumpi(log_after_topics)
    // stack: topics_ptr, num_topics, topics, data_len, data_offset, retdest
    // Store the first topic.
    DUP3 DUP2
    %mstore_kernel(@SEGMENT_LOGS_DATA)
    %increment
    %stack (curr_topic_ptr, num_topics, topic1) -> (curr_topic_ptr, num_topics)
    DUP2 %eq_const(1)
    %jumpi(log_after_topics)
    // stack: curr_topic_ptr, num_topics, remaining_topics, data_len, data_offset, retdest
    // Store the second topic.
    DUP3 DUP2
    %mstore_kernel(@SEGMENT_LOGS_DATA)
    %increment
    %stack (curr_topic_ptr, num_topics, topic2) -> (curr_topic_ptr, num_topics)
    DUP2 %eq_const(2)
    %jumpi(log_after_topics)
    // stack: curr_topic_ptr, num_topics, remaining_topics, data_len, data_offset, retdest
    // Store the third topic.
    DUP3 DUP2
    %mstore_kernel(@SEGMENT_LOGS_DATA)
    %increment
    %stack (curr_topic_ptr, num_topics, topic3) -> (curr_topic_ptr, num_topics)
    DUP2 %eq_const(3)
    %jumpi(log_after_topics)
    // stack: curr_topic_ptr, num_topics, remaining_topic, data_len, data_offset, retdest
    // Store the fourth topic.
    DUP3 DUP2
    %mstore_kernel(@SEGMENT_LOGS_DATA)
    %increment
    %stack (data_len_ptr, num_topics, topic4) -> (data_len_ptr, num_topics)
    DUP2 %eq_const(4)
    %jumpi(log_after_topics)
    // Invalid num_topics.
    PANIC

log_after_topics:
    // stack: data_len_ptr, num_topics, data_len, data_offset, retdest
    // Compute RLP length of the log.
    DUP3
    // stack: data_len, data_len_ptr, num_topics, data_len, data_offset, retdest
    DUP5 SWAP1
    %rlp_data_len
    // stack: rlp_data_len, data_len_ptr, num_topics, data_len, data_offset, retdest
    DUP3
    // stack: num_topics, rlp_data_len, data_len_ptr, num_topics, data_len, data_offset, retdest
    // Each topic is encoded with 1+32 bytes.
    %mul_const(33)
    %rlp_list_len
    // stack: rlp_topics_len, rlp_data_len, data_len_ptr, num_topics, data_len, data_offset, retdest
    ADD
    // The address is encoded with 1+20 bytes.
    %add_const(21)
    // stack: log_payload_len, data_len_ptr, num_topics, data_len, data_offset, retdest
    %mload_global_metadata(@GLOBAL_METADATA_LOGS_DATA_LEN)
    DUP2 SWAP1
    // stack: log_ptr, log_payload_len, log_payload_len, data_len_ptr, num_topics, data_len, data_offset, retdest
    %mstore_kernel(@SEGMENT_LOGS_DATA)
    // stack: log_payload_len, data_len_ptr, num_topics, data_len, data_offset, retdest
    %rlp_list_len
    // stack: rlp_log_len, data_len_ptr, num_topics, data_len, data_offset, retdest
    %mload_global_metadata(@GLOBAL_METADATA_LOGS_PAYLOAD_LEN)
    // Add payload length and logs_data_len to journal.
    DUP1 %mload_global_metadata(@GLOBAL_METADATA_LOGS_DATA_LEN) %journal_add_log
    ADD
    %mstore_global_metadata(@GLOBAL_METADATA_LOGS_PAYLOAD_LEN)
    // stack: data_len_ptr, num_topics, data_len, data_offset, retdest
    // Store data_len.
    DUP3 DUP2
    %mstore_kernel(@SEGMENT_LOGS_DATA)
    %increment
    // stack: data_ptr, num_topics, data_len, data_offset, retdest
    SWAP1 POP
    // stack: data_ptr, data_len, data_offset, retdest
    DUP1 SWAP2
    // stack: data_len, data_ptr, data_ptr, data_offset, retdest
    ADD
    // stack: next_log_ptr, data_ptr, data_offset, retdest
    SWAP1
    // stack: data_ptr, next_log_ptr, data_offset, retdest

store_log_data_loop:
    // stack: cur_data_ptr, next_log_ptr, cur_data_offset, retdest
    DUP2 DUP2 EQ
    // stack: cur_data_ptr == next_log_ptr, cur_data_ptr, next_log_ptr, cur_data_offset, retdest
    %jumpi(store_log_data_loop_end)
    // stack: cur_data_ptr, next_log_ptr, cur_data_offset, retdest
    DUP3
    %mload_current(@SEGMENT_MAIN_MEMORY)
    // stack: cur_data, cur_data_ptr, next_log_ptr, cur_data_offset, retdest
    // Store current data byte.
    DUP2
    %mstore_kernel(@SEGMENT_LOGS_DATA)
    // stack: cur_data_ptr, next_log_ptr, cur_data_offset, retdest
    SWAP2 %increment SWAP2
    // stack: cur_data_ptr, next_log_ptr, next_data_offset, retdest
    %increment
    %jump(store_log_data_loop)

store_log_data_loop_end:
    // stack: cur_data_ptr, next_log_ptr, cur_data_offset, retdest
    POP
    %mstore_global_metadata(@GLOBAL_METADATA_LOGS_DATA_LEN)
    POP
    JUMP

rlp_data_len:
    // stack: data_len, data_ptr, retdest
    DUP1 ISZERO %jumpi(data_single_byte) // data will be encoded with a single byte
    DUP1 PUSH 1 EQ %jumpi(one_byte_data) // data is encoded with either 1 or 2 bytes
    // If we are here, data_len >= 2, and we can use rlp_list_len to determine the encoding length
    %rlp_list_len
    // stack: rlp_data_len, data_ptr, retdest
    SWAP1 POP SWAP1
    JUMP

data_single_byte:
    // stack: data_len, data_ptr, retdest
    %pop2
    PUSH 1
    SWAP1
    JUMP

one_byte_data:
    // stack: data_len, data_ptr, retdest
    DUP2
    %mload_current(@SEGMENT_MAIN_MEMORY)
    // stack: data_byte, data_len, data_ptr, retdest
    %lt_const(0x80) %jumpi(data_single_byte) // special byte that only requires one byte to be encoded
    %pop2
    PUSH 2 SWAP1
    JUMP

%macro rlp_data_len
    // stack: data_len, data_ptr
    %stack (data_len, data_ptr) -> (data_len, data_ptr, %%after)
    %jump(rlp_data_len)
%%after:
%endmacro
