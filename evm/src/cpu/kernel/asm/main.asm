global main:
    // First, load all MPT data from the prover.
    PUSH txn_loop
    %jump(load_all_mpts)

hash_initial_tries:
    %mpt_hash_state_trie   %mstore_global_metadata(@GLOBAL_METADATA_STATE_TRIE_DIGEST_BEFORE)
    %mpt_hash_txn_trie     %mstore_global_metadata(@GLOBAL_METADATA_TXN_TRIE_DIGEST_BEFORE)
    %mpt_hash_receipt_trie %mstore_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_DIGEST_BEFORE)

txn_loop:
    // If the prover has no more txns for us to process, halt.
    PROVER_INPUT(end_of_txns)
    %jumpi(hash_final_tries)

    // Call route_txn. When we return, continue the txn loop.
    PUSH txn_loop
    %jump(route_txn)

hash_final_tries:
    %mpt_hash_state_trie   %mstore_global_metadata(@GLOBAL_METADATA_STATE_TRIE_DIGEST_AFTER)
    %mpt_hash_txn_trie     %mstore_global_metadata(@GLOBAL_METADATA_TXN_TRIE_DIGEST_AFTER)
    %mpt_hash_receipt_trie %mstore_global_metadata(@GLOBAL_METADATA_RECEIPT_TRIE_DIGEST_AFTER)
    %jump(halt)
