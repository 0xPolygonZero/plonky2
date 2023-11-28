global main:
    // First, initialise the shift table
    %shift_table_init

    // Initialize the block bloom filter
    %initialize_block_bloom

    // Second, load all MPT data from the prover.
    PUSH hash_initial_tries
    %jump(load_all_mpts)

global hash_initial_tries:
    %smt_hash_state        %mload_global_metadata(@GLOBAL_METADATA_STATE_TRIE_DIGEST_BEFORE)    %assert_eq
    %mpt_hash_txn_trie     %mload_global_metadata(@GLOBAL_METADATA_TXN_TRIE_DIGEST_BEFORE)      %assert_eq
    %mpt_hash_receipt_trie %mload_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_DIGEST_BEFORE)  %assert_eq

global start_txn:
    // stack: (empty)
    // The special case of an empty trie (i.e. for the first transaction)
    // is handled outside of the kernel.
    %mload_global_metadata(@GLOBAL_METADATA_TXN_NUMBER_BEFORE)
    // stack: txn_nb
    %mload_global_metadata(@GLOBAL_METADATA_BLOCK_GAS_USED_BEFORE)
    // stack: init_used_gas, txn_nb
    DUP2 %scalar_to_rlp
    // stack: txn_counter, init_gas_used, txn_nb
    DUP1 %num_bytes %mul_const(2)
    // stack: num_nibbles, txn_counter, init_gas_used, txn_nb
    SWAP2
    // stack: init_gas_used, txn_counter, num_nibbles, txn_nb

    // If the prover has no txn for us to process, halt.
    PROVER_INPUT(no_txn)
    %jumpi(execute_withdrawals)

    // Call route_txn. When we return, we will process the txn receipt.
    PUSH txn_after
    // stack: retdest, prev_gas_used, txn_counter, num_nibbles, txn_nb
    DUP4 DUP4 %increment_bounded_rlp
    %stack (next_txn_counter, next_num_nibbles, retdest, prev_gas_used, txn_counter, num_nibbles) -> (txn_counter, num_nibbles, retdest, prev_gas_used, txn_counter, num_nibbles, next_txn_counter, next_num_nibbles)
    %jump(route_txn)

global txn_after:
    // stack: success, leftover_gas, cur_cum_gas, prev_txn_counter, prev_num_nibbles, txn_counter, num_nibbles, txn_nb
    %process_receipt
    // stack: new_cum_gas, txn_counter, num_nibbles, txn_nb
    SWAP3 %increment SWAP3

global execute_withdrawals:
    // stack: cum_gas, txn_counter, num_nibbles, txn_nb
    %withdrawals
global hash_final_tries:
    // stack: cum_gas, txn_counter, num_nibbles, txn_nb
    // Check that we end up with the correct `cum_gas`, `txn_nb` and bloom filter.
    %mload_global_metadata(@GLOBAL_METADATA_BLOCK_GAS_USED_AFTER) %assert_eq
    DUP3 %mload_global_metadata(@GLOBAL_METADATA_TXN_NUMBER_AFTER) %assert_eq
    %pop3
    %check_metadata_block_bloom
    %smt_hash_state        %mload_global_metadata(@GLOBAL_METADATA_STATE_TRIE_DIGEST_AFTER)     %assert_eq
    %mpt_hash_txn_trie     %mload_global_metadata(@GLOBAL_METADATA_TXN_TRIE_DIGEST_AFTER)       %assert_eq
    %mpt_hash_receipt_trie %mload_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_DIGEST_AFTER)   %assert_eq
    %jump(halt)

initialize_block_bloom:
    // stack: retdest
    PUSH 0 PUSH 8 PUSH 0

initialize_bloom_loop:
    // stack: i, len, offset, retdest
    DUP2 DUP2 EQ %jumpi(initialize_bloom_loop_end)
    PUSH 32 // Bloom word length
    // stack: word_len, i, len, offset, retdest
    // Load the next `block_bloom_before` word.
    DUP2 %add_const(8) %mload_kernel(@SEGMENT_GLOBAL_BLOCK_BLOOM)
    // stack: bloom_word, word_len, i, len, offset, retdest
    DUP5 PUSH @SEGMENT_BLOCK_BLOOM PUSH 0 // Bloom word address in SEGMENT_BLOCK_BLOOM
    %mstore_unpacking
    // stack: new_offset, i, len, old_offset, retdest
    SWAP3 POP %increment
    // stack: i, len, new_offset, retdest
    %jump(initialize_bloom_loop)

initialize_bloom_loop_end:
    // stack: len, len, offset, retdest
    %pop3
    JUMP
    
%macro initialize_block_bloom
    // stack: (empty)
    PUSH %%after
    %jump(initialize_block_bloom)
%%after:
%endmacro

check_metadata_block_bloom:
    // stack: retdest
    PUSH 0 PUSH 8 PUSH 0

check_bloom_loop:
    // stack: i, len, offset, retdest
    DUP2 DUP2 EQ %jumpi(check_bloom_loop_end)
    PUSH 32 // Bloom word length
    // stack: word_len, i, len, offset, retdest
    DUP4 PUSH @SEGMENT_BLOCK_BLOOM PUSH 0
    %mload_packing
    // stack: bloom_word, i, len, offset, retdest
    DUP2 %add_const(16) %mload_kernel(@SEGMENT_GLOBAL_BLOCK_BLOOM) %assert_eq
    // stack: i, len, offset, retdest
    %increment SWAP2 %add_const(32) SWAP2
    // stack: i+1, len, new_offset, retdest
    %jump(check_bloom_loop)

check_bloom_loop_end:
    // stack: len, len, offset, retdest
    %pop3
    JUMP

%macro check_metadata_block_bloom
    PUSH %%after
    %jump(check_metadata_block_bloom)
%%after:
%endmacro
