// Return the next context ID, and record the old context ID in the new one's
// @CTX_METADATA_PARENT_CONTEXT field. Does not actually enter the new context.
%macro create_context
    %next_context_id
    GET_CONTEXT
    %stack (ctx, next_ctx)
       -> (next_ctx, @SEGMENT_NORMALIZED_TXN, @CTX_METADATA_PARENT_CONTEXT,
           ctx, next_ctx)
    MSTORE_GENERAL
    // stack: next_ctx
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
