global mpt_load_state_trie_value:
    // stack: retdest

    // Load and append the nonce and balance.
    PROVER_INPUT(mpt) %append_to_trie_data
    PROVER_INPUT(mpt) %append_to_trie_data

    // Now increment the trie data size by 2, to leave room for our storage trie
    // pointer and code hash fields, before calling load_mpt which will append
    // our storage trie data.
    %get_trie_data_size
    // stack: storage_trie_ptr_ptr, retdest
    DUP1 %add_const(2)
    // stack: storage_trie_ptr, storage_trie_ptr_ptr, retdest
    %set_trie_data_size
    // stack: storage_trie_ptr_ptr, retdest

    %load_mpt(mpt_load_storage_trie_value)
    // stack: storage_trie_ptr, storage_trie_ptr_ptr, retdest
    DUP2 %mstore_trie_data
    // stack: storage_trie_ptr_ptr, retdest
    %increment
    // stack: code_hash_ptr, retdest
    PROVER_INPUT(mpt)
    // stack: code_hash, code_hash_ptr, retdest
    SWAP1 %mstore_trie_data
    // stack: retdest
    JUMP

global mpt_load_txn_trie_value:
    // stack: retdest
    PANIC // TODO

global mpt_load_receipt_trie_value:
    // stack: retdest
    PANIC // TODO

global mpt_load_storage_trie_value:
    // stack: retdest
    PANIC // TODO
