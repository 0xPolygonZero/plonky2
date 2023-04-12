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

global make_default_account:
    PANIC // TODO

// Create a copy of the given account. The copy can then safely be mutated as
// needed, while leaving the original account data untouched.
//
// This writes the new account's data to MPT data, but does not register the new
// account in the state trie.
//
// Pre stack: old_account_ptr, retdest
// Post stack: new_account_ptr
global make_account_copy:
    // stack: old_account_ptr, retdest
    %get_trie_data_size // pointer to new account we're about to create
    // stack: new_account_ptr, old_account_ptr, retdest

    DUP2                %mload_trie_data %append_to_trie_data
    DUP2  %add_const(1) %mload_trie_data %append_to_trie_data
    DUP2  %add_const(3) %mload_trie_data %append_to_trie_data
    SWAP1 %add_const(4) %mload_trie_data %append_to_trie_data

    // stack: new_account_ptr, retdest
    SWAP1
    JUMP

// Convenience macro to call make_account_copy and return where we left off.
%macro make_account_copy
    %stack (old_account_ptr) -> (old_account_ptr, %%after)
    %jump(make_account_copy)
%%after:
%endmacro
