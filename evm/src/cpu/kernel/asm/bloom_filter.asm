/// Implementation of Bloom filters for logs.

// Adds a Bloom entry to the transaction Bloom filter and the block Bloom filter.
//
// This is calculated by taking the least significant 11 bits from
// the first 3 16-bit bytes of the keccak_256 hash of bloom_entry.
add_to_bloom:
    // stack: is_topic, bloom_entry, retdest
    %compute_entry_hash
    // stack: hash, retdest
    DUP1
    // stack: hash, hash, retdest
    %shr_const(240)
    // stack: hahs_shft_240, hash, retdest
    %bloom_byte_indices
    // stack: byte_index, byte_bit_index, hash, retdest
    %bloom_write_bit
    // stack: hash, retdest

    // We shift the hash by 16 bits and repeat.
    DUP1 %shr_const(224)
    // stack: hash_shft_224, hash, retdest
    %bloom_byte_indices
    // stack: byte_index, byte_bit_index, hash, retdest
    %bloom_write_bit
    // stack: hash, retdest

    // We shift again the hash by 16 bits and repeat.
    %shr_const(208)
    // stack: hash_shft_208, retdest
    %bloom_byte_indices
    // stack: byte_index, byte_bit_index, retdest
    %bloom_write_bit
    // stack: retdest
    JUMP

// The LOGS segment is [log0_ptr, log1_ptr...]. logs_len is a global metadata for the number of logs.
// A log in the LOGS_DATA segment is [log_payload_len, address, num_topics, [topics], data_len, [data]].
global logs_bloom:
    // stack: retdest
    %mload_global_metadata(@GLOBAL_METADATA_LOGS_LEN)
    // stack: logs_len, retdest
    PUSH 0

logs_bloom_loop:
    // stack: i, logs_len, retdest
    DUP2 DUP2 EQ
    // stack: i == logs_len, i, logs_len, retdest
    %jumpi(logs_bloom_end)
    // stack: i, logs_len, retdest
    DUP1
    %mload_kernel(@SEGMENT_LOGS)
    // stack: log_payload_len_ptr, i, logs_len, retdest
    
    // Add address to bloom filter.
    %increment
    // stack: addr_ptr, i, logs_len, retdest
    DUP1
    %mload_kernel(@SEGMENT_LOGS_DATA)
    // stack: addr, addr_ptr, i, logs_len, retdest
    PUSH 0
    // stack: is_topic, addr, addr_ptr, i, logs_len, retdest
    %add_to_bloom
    // stack: addr_ptr, i, logs_len, retdest
    %increment
    // stack: num_topics_ptr, i, logs_len, retdest
    DUP1
    %mload_kernel(@SEGMENT_LOGS_DATA)
    // stack: num_topics, num_topics_ptr, i, logs_len, retdest
    SWAP1 %increment
    // stack: topics_ptr, num_topics, i, logs_len, retdest
    PUSH 0

logs_bloom_topic_loop:
    // stack: j, topics_ptr, num_topics, i, logs_len, retdest
    DUP3 DUP2 EQ
    // stack: j == num_topics, j, topics_ptr, num_topics, i, logs_len, retdest
    %jumpi(logs_bloom_topic_end)
    DUP2 DUP2 ADD
    // stack: curr_topic_ptr, j, topics_ptr, num_topics, i, logs_len, retdest
    %mload_kernel(@SEGMENT_LOGS_DATA)
    // stack: topic, j, topics_ptr, num_topics, i, logs_len, retdest
    PUSH 1
    // stack: is_topic, topic, j, topics_ptr, num_topics, i, logs_len, retdest
    %add_to_bloom
    // stack: j, topics_ptr, num_topics, i, logs_len, retdest
    %increment
    %jump(logs_bloom_topic_loop)

logs_bloom_topic_end:
    // stack: num_topics, topics_ptr, num_topics, i, logs_len, retdest
    %pop3
    %increment
    %jump(logs_bloom_loop)

logs_bloom_end:
    // stack: logs_len, logs_len, retdest
    %pop2
    JUMP

%macro compute_entry_hash
    // stack: is_topic, bloom_entry
    ISZERO
    %jumpi(%%compute_entry_hash_address)
    // stack: bloom_entry
    %keccak256_word(32)
    // stack: topic_hash
    %jump(%%after)

%%compute_entry_hash_address:
    // stack: bloom_entry
    %keccak256_word(20)
    // stack: address_hash

%%after:
%endmacro

%macro add_to_bloom
    %stack (is_topic, bloom_entry) -> (is_topic, bloom_entry, %%after)
    %jump(add_to_bloom)

%%after:
%endmacro

// Computes the byte index and bit index within to update the Bloom filter with.
// The hash value must be properly shifted prior calling this macro.
%macro bloom_byte_indices
    // stack: hash
    %and_const(0x07FF)
    PUSH 0x07FF
    SUB
    // stack: bit_index
    DUP1
    %and_const(0x7)
    SWAP1
    %shr_const(0x3)
    // stack: byte_index, byte_bit_index
%endmacro


// Updates the corresponding bloom filter byte with provided bit.
// Also updates the block bloom filter.
%macro bloom_write_bit
    // stack: byte_index, byte_bit_index
    PUSH 1
    DUP3
    // stack: byte_bit_index, 1, byte_index, byte_bit_index
    PUSH 7 SUB
    SHL
    // Updates the current txn bloom filter.
    SWAP2 POP DUP1
    %mload_kernel(@SEGMENT_TXN_BLOOM)
    // stack: old_bloom_byte, byte_index, one_shifted_by_index
    DUP3 OR
    // stack: new_bloom_byte, byte_index, one_shifted_by_index
    SWAP1
    %mstore_kernel(@SEGMENT_TXN_BLOOM)
    // stack: one_shifted_by_index
    POP
    // stack: empty
%endmacro
    


