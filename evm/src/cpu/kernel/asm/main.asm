global main:
    // First, hash the kernel code
    %mload_global_metadata(@GLOBAL_METADATA_KERNEL_LEN)
    PUSH 0
    // stack: addr, len
    KECCAK_GENERAL
    // stack: hash
    %mload_global_metadata(@GLOBAL_METADATA_KERNEL_HASH)
    // stack: expected_hash, hash
    %assert_eq

    // Initialise the shift table
    %shift_table_init

    // Initialize the RLP DATA pointer to its initial position (ctx == virt == 0, segment = RLP)
    PUSH @SEGMENT_RLP_RAW
    %mstore_global_metadata(@GLOBAL_METADATA_RLP_DATA_SIZE)

    // Encode constant nodes
    %initialize_rlp_segment
   
    // Initialize the state, transaction and receipt trie root pointers.
    PROVER_INPUT(trie_ptr::state)
    %mstore_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT)
    PROVER_INPUT(trie_ptr::txn)
    %mstore_global_metadata(@GLOBAL_METADATA_TXN_TRIE_ROOT)
    PROVER_INPUT(trie_ptr::receipt)
    %mstore_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_ROOT)

global hash_initial_tries:
    // We compute the length of the trie data segment in `mpt_hash` so that we
    // can check the value provided by the prover.
    // We initialize the segment length with 1 because the segment contains 
    // the null pointer `0` when the tries are empty.
    PUSH 2
    %smt_hash_state %mload_global_metadata(@GLOBAL_METADATA_STATE_TRIE_DIGEST_BEFORE) %assert_eq
    // stack: trie_data_len
    %mpt_hash_txn_trie     %mload_global_metadata(@GLOBAL_METADATA_TXN_TRIE_DIGEST_BEFORE)      %assert_eq
    // stack: trie_data_len
    %mpt_hash_receipt_trie %mload_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_DIGEST_BEFORE)  %assert_eq
    // stack: trie_data_full_len
    %mstore_global_metadata(@GLOBAL_METADATA_TRIE_DATA_SIZE)

global start_txn:
    // stack: (empty)
    // The special case of an empty trie (i.e. for the first transaction)
    // is handled outside of the kernel.
    %mload_global_metadata(@GLOBAL_METADATA_TXN_NUMBER_BEFORE)
    // stack: txn_nb
    DUP1 %scalar_to_rlp
    // stack: txn_counter, txn_nb
    DUP1 %num_bytes %mul_const(2)
    // stack: num_nibbles, txn_counter, txn_nb
    %increment_bounded_rlp
    // stack: txn_counter, num_nibbles, next_txn_counter, next_num_nibbles,  txn_nb
    %mload_global_metadata(@GLOBAL_METADATA_BLOCK_GAS_USED_BEFORE)

    // stack: init_gas_used, txn_counter, num_nibbles, next_txn_counter, next_num_nibbles, txn_nb

    // If the prover has no txn for us to process, halt.
    PROVER_INPUT(no_txn)
    %jumpi(execute_withdrawals)

    // Call route_txn. When we return, we will process the txn receipt.
    PUSH txn_after
    // stack: retdest, prev_gas_used, txn_counter, num_nibbles, next_txn_counter, next_num_nibbles, txn_nb
    DUP4 DUP4

    %jump(route_txn)

global txn_after:
    // stack: success, leftover_gas, cur_cum_gas, prev_txn_counter, prev_num_nibbles, txn_counter, num_nibbles, txn_nb
    %process_receipt
    // stack: new_cum_gas, txn_counter, num_nibbles, txn_nb
    SWAP3 %increment SWAP3
    %jump(execute_withdrawals_post_stack_op)

global execute_withdrawals:
    // stack: cum_gas, txn_counter, num_nibbles, next_txn_counter, next_num_nibbles, txn_nb
    %stack (cum_gas, txn_counter, num_nibbles, next_txn_counter, next_num_nibbles) -> (cum_gas, txn_counter, num_nibbles)
execute_withdrawals_post_stack_op:
    %withdrawals

global wtf1:
    PUSH 0x095e7baea6a6c7c4c2dfeb977efac326af552d87
    // stack: addr
    DUP1 %key_nonce %smt_read_state %mload_trie_data
global wtf2:
    POP
    DUP1 %key_balance %smt_read_state %mload_trie_data
global wtf3:
    POP
    DUP1 %key_code_length %smt_read_state %mload_trie_data
global wtf4:
    POP
    %stack (addr) -> (addr, 0)
    %key_storage %smt_read_state %mload_trie_data
global wtf5:
    POP
    PUSH 0xa94f5374fce5edbc8e2a8697c15331677e6ebf0b
    // stack: addr
    DUP1 %key_nonce %smt_read_state %mload_trie_data
global wtf6:
    POP
    DUP1 %key_balance %smt_read_state %mload_trie_data
global wtf7:
    POP
    DUP1 %key_code_length %smt_read_state %mload_trie_data
global wtf8:
    POP POP
    PUSH 0xd2571607e241ecf590ed94b12d87c94babe36db6
    // stack: addr
    DUP1 %key_nonce %smt_read_state %mload_trie_data
global wtf9:
    POP
    DUP1 %key_balance %smt_read_state %mload_trie_data
global wtf10:
    POP
    DUP1 %key_code_length %smt_read_state %mload_trie_data
global wtf11:
    POP POP

global hash_final_tries:
    // stack: cum_gas, txn_counter, num_nibbles, txn_nb
    // Check that we end up with the correct `cum_gas`, `txn_nb` and bloom filter.
    %mload_global_metadata(@GLOBAL_METADATA_BLOCK_GAS_USED_AFTER)
global lol1:
    %assert_eq
    DUP3 %mload_global_metadata(@GLOBAL_METADATA_TXN_NUMBER_AFTER)
global lol2:
    %assert_eq
    %pop3
    PUSH 1 // initial trie data length 
    %smt_hash_state        %mload_global_metadata(@GLOBAL_METADATA_STATE_TRIE_DIGEST_AFTER)
global lol3:
    %assert_eq
    %mpt_hash_txn_trie     %mload_global_metadata(@GLOBAL_METADATA_TXN_TRIE_DIGEST_AFTER)       %assert_eq
    %mpt_hash_receipt_trie %mload_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_DIGEST_AFTER)   %assert_eq
    // We don't need the trie data length here.
    POP
    %jump(halt)
