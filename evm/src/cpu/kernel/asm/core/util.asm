// Return the next context ID, and record the old context ID in the new one's
// @CTX_METADATA_PARENT_CONTEXT field. Does not actually enter the new context.
%macro create_context
    // stack: (empty)
    %next_context_id
    %set_new_ctx_parent_ctx
    // stack: new_ctx
%endmacro

// Get and increment @GLOBAL_METADATA_LARGEST_CONTEXT to determine the next context ID.
%macro next_context_id
    // stack: (empty)
    %mload_global_metadata(@GLOBAL_METADATA_LARGEST_CONTEXT)
    %increment
    // stack: new_ctx
    DUP1
    %mstore_global_metadata(@GLOBAL_METADATA_LARGEST_CONTEXT)
    // stack: new_ctx
%endmacro

// Returns whether the current transaction is a contract creation transaction.
%macro is_contract_creation
    // stack: (empty)
    %mload_txn_field(@TXN_FIELD_TO)
    // stack: to
    ISZERO
    // If there is no "to" field, then this is a contract creation.
    // stack: to == 0
%endmacro

// Returns 1 if the account is non-existent, 0 otherwise.
%macro is_non_existent
    // stack: addr
    %mpt_read_state_trie
    ISZERO
%endmacro

// Returns 1 if the account is empty, 0 otherwise.
%macro is_empty
    // stack: addr
    %mpt_read_state_trie
    // stack: account_ptr
    DUP1 ISZERO %jumpi(%%false)
    // stack: account_ptr
    DUP1 %mload_trie_data
    // stack: nonce, account_ptr
    ISZERO %not_bit %jumpi(%%false)
    %increment DUP1 %mload_trie_data
    // stack: balance, balance_ptr
    ISZERO %not_bit %jumpi(%%false)
    %add_const(2) %mload_trie_data
    // stack: code_hash
    PUSH @EMPTY_STRING_HASH
    EQ
    %jump(%%after)
%%false:
    // stack: account_ptr
    POP
    PUSH 0
%%after:
%endmacro

// Returns 1 if the account is dead (i.e., empty or non-existent), 0 otherwise.
%macro is_dead
    // stack: addr
    DUP1 %is_non_existent
    SWAP1 %is_empty
    ADD // OR
%endmacro
