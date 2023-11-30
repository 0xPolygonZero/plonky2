global main:
    // First, hash the kernel code
    %mload_global_metadata(@GLOBAL_METADATA_KERNEL_LEN)
    PUSH 0
    PUSH 0
    PUSH 0
    // stack: context, segment, virt, len
    KECCAK_GENERAL
    // stack: hash
    %mload_global_metadata(@GLOBAL_METADATA_KERNEL_HASH)
    // stack: expected_hash, hash
    %assert_eq

    // Initialise the shift table
    %shift_table_init

    // Second, load all MPT data from the prover.
    PUSH hash_initial_tries
    %jump(load_all_mpts)

global hash_initial_tries:
    %mpt_hash_state_trie   %mload_global_metadata(@GLOBAL_METADATA_STATE_TRIE_DIGEST_BEFORE)    %assert_eq
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
    %mpt_hash_state_trie   %mload_global_metadata(@GLOBAL_METADATA_STATE_TRIE_DIGEST_AFTER)     %assert_eq
    %mpt_hash_txn_trie     %mload_global_metadata(@GLOBAL_METADATA_TXN_TRIE_DIGEST_AFTER)       %assert_eq
    %mpt_hash_receipt_trie %mload_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_DIGEST_AFTER)   %assert_eq
    %jump(halt)
