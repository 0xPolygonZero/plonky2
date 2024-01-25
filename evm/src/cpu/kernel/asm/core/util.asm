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
    %add_const(0x10000000000000000) // scale each context by 2^64
    // stack: new_ctx
    DUP1
    %mstore_global_metadata(@GLOBAL_METADATA_LARGEST_CONTEXT)
    // stack: new_ctx
%endmacro

// Returns whether the current transaction is a contract creation transaction.
%macro is_contract_creation
    // stack: (empty)
    %mload_global_metadata(@GLOBAL_METADATA_CONTRACT_CREATION)
%endmacro

%macro is_precompile
    // stack: addr
    DUP1 %ge_const(@ECREC) SWAP1 %le_const(@BLAKE2_F)
    // stack: addr>=1, addr<=9
    MUL // Cheaper than AND
%endmacro

// Returns 1 if the account is non-existent, 0 otherwise.
%macro is_non_existent
    // stack: addr
    %mpt_read_state_trie ISZERO
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
    OR
%endmacro

// Gets the size of the stack _before_ the macro is run
// WARNING: this macro is side-effecting. It writes the current stack length to offset
// `CTX_METADATA_STACK_SIZE`, segment `SEGMENT_CONTEXT_METADATA` in the current context. But I can't
// imagine it being an issue unless someone's doing something dumb.
%macro stack_length
    // stack: (empty)
    GET_CONTEXT
    // stack: current_ctx
    // It seems odd to switch to the context that we are already in. We do this because SET_CONTEXT
    // saves the stack length of the context we are leaving in its metadata segment.
    SET_CONTEXT
    // stack: (empty)
    // We can now read this stack length from memory.
    %mload_context_metadata(@CTX_METADATA_STACK_SIZE)
    // stack: stack_length
%endmacro
