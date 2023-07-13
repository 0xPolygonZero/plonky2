global main:
    // First, initialise the shift table
    %shift_table_init

    // Second, load all MPT data from the prover.
    PUSH hash_initial_tries
    %jump(load_all_mpts)

global hash_initial_tries:
    %mpt_hash_state_trie   %mstore_global_metadata(@GLOBAL_METADATA_STATE_TRIE_DIGEST_BEFORE)
    %mpt_hash_txn_trie     %mstore_global_metadata(@GLOBAL_METADATA_TXN_TRIE_DIGEST_BEFORE)
    %mpt_hash_receipt_trie %mstore_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_DIGEST_BEFORE)

global txn_loop:
    // If the prover has no more txns for us to process, halt.
    PROVER_INPUT(end_of_txns)
    %jumpi(hash_final_tries)

    %zero_rlp
    %zero_metadata

    // Call route_txn. When we return, continue the txn loop.
    PUSH txn_loop
    %jump(route_txn)

global hash_final_tries:
    %withdrawals
    %mpt_hash_state_trie   %mstore_global_metadata(@GLOBAL_METADATA_STATE_TRIE_DIGEST_AFTER)
    %mpt_hash_txn_trie     %mstore_global_metadata(@GLOBAL_METADATA_TXN_TRIE_DIGEST_AFTER)
    %mpt_hash_receipt_trie %mstore_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_DIGEST_AFTER)
    %jump(halt)

%macro zero_rlp
    PUSH 0
%%loop:
    DUP1 %eq_const(5000) %jumpi(%%end)
    PUSH 0 DUP2 PUSH 10 PUSH 0 MSTORE_GENERAL
    PUSH 0 DUP2 PUSH 11 PUSH 0 MSTORE_GENERAL
    PUSH 0 DUP2 PUSH 12 PUSH 0 MSTORE_GENERAL
    %increment
    %jump(%%loop)
%%end:
    POP
%endmacro

%macro zero_metadata
    PUSH 0 %mstore_global_metadata(@GLOBAL_METADATA_MEMORY_SIZE)
    PUSH 0 %mstore_global_metadata(@GLOBAL_METADATA_REFUND_COUNTER)
    PUSH 0 %mstore_global_metadata(@GLOBAL_METADATA_ACCESSED_ADDRESSES_LEN)
    PUSH 0 %mstore_global_metadata(@GLOBAL_METADATA_ACCESSED_STORAGE_KEYS_LEN)
    PUSH 0 %mstore_global_metadata(@GLOBAL_METADATA_SELFDESTRUCT_LIST_LEN)
    PUSH 0 %mstore_global_metadata(@GLOBAL_METADATA_JOURNAL_LEN)
    PUSH 0 %mstore_global_metadata(@GLOBAL_METADATA_JOURNAL_DATA_LEN)
    PUSH 0 %mstore_global_metadata(@GLOBAL_METADATA_CURRENT_CHECKPOINT)
    PUSH 0 %mstore_global_metadata(@GLOBAL_METADATA_TOUCHED_ADDRESSES_LEN)
    PUSH 0 %mstore_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_DATA_COST)
    PUSH 0 %mstore_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_RLP_START)
    PUSH 0 %mstore_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_RLP_LEN)
    PUSH 0 %mstore_global_metadata(@GLOBAL_METADATA_CONTRACT_CREATION)
    PUSH 0 %mstore_global_metadata(@GLOBAL_METADATA_IS_PRECOMPILE_FROM_EOA)
    PUSH 0 %mstore_global_metadata(@GLOBAL_METADATA_CALL_STACK_DEPTH)
%endmacro
