global main:
    // First, load all MPT data from the prover.
    PUSH txn_loop
    %jump(load_all_mpts)

hash_initial_tries:
    // TODO: Hash each trie and set @GLOBAL_METADATA_STATE_TRIE_DIGEST_BEFORE, etc.

txn_loop:
    // If the prover has no more txns for us to process, halt.
    PROVER_INPUT(end_of_txns)
    %jumpi(hash_final_tries)

    // Call route_txn. When we return, continue the txn loop.
    PUSH txn_loop
    %jump(route_txn)

hash_final_tries:
    // TODO: Hash each trie and set @GLOBAL_METADATA_STATE_TRIE_DIGEST_AFTER, etc.
    %jump(halt)
