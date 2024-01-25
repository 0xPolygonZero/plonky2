// Return a pointer to the current account's data in the state trie.
%macro current_account_data
    %address %mpt_read_state_trie
    // stack: account_ptr
    // account_ptr should be non-null as long as the prover provided the proper
    // Merkle data. But a bad prover may not have, and we don't want return a
    // null pointer for security reasons.
    DUP1 ISZERO %jumpi(panic)
    // stack: account_ptr
%endmacro

// Returns a pointer to the root of the storage trie associated with the current account.
%macro current_storage_trie
    // stack: (empty)
    %current_account_data
    // stack: account_ptr
    %add_const(2)
    // stack: storage_root_ptr_ptr
    %mload_trie_data
    // stack: storage_root_ptr
%endmacro
