// After the transaction data has been parsed into a normalized set of fields
// (see NormalizedTxnField), this routine processes the transaction.

// TODO: Save checkpoints in @CTX_METADATA_STATE_TRIE_CHECKPOINT_PTR and @SEGMENT_STORAGE_TRIE_CHECKPOINT_PTRS.

// Pre stack: retdest
// Post stack: (empty)
global process_normalized_txn:
    // stack: retdest
    PUSH validate
    %jump(intrinsic_gas)

global validate:
    // stack: intrinsic_gas, retdest
    // TODO: Check signature? (Or might happen in type_0.asm etc.)
    // TODO: Assert nonce is correct.
    // TODO: Assert sender has no code.
    POP // TODO: Assert gas_limit >= intrinsic_gas.
    // stack: retdest

global charge_gas:
    // TODO: Deduct gas limit from sender (some gas may be refunded later).

    PUSH 0 // TODO: Push sender.
    %increment_nonce

global process_based_on_type:
    %is_contract_creation
    %jumpi(process_contract_creation_txn)
    %jump(process_message_txn)

global process_contract_creation_txn:
    // stack: retdest
    // Push the code address & length onto the stack, then call `create`.
    %mload_txn_field(@TXN_FIELD_DATA_LEN)
    // stack: code_len, retdest
    PUSH 0
    // stack: code_offset, code_len, retdest
    PUSH @SEGMENT_TXN_DATA
    // stack: code_segment, code_offset, code_len, retdest
    PUSH 0 // context
    // stack: CODE_ADDR, code_len, retdest
    %jump(create)

global process_message_txn:
    // stack: retdest
    %mload_txn_field(@TXN_FIELD_VALUE)
    %mload_txn_field(@TXN_FIELD_TO)
    %mload_txn_field(@TXN_FIELD_ORIGIN)
    // stack: from, to, amount, retdest
    %transfer_eth
    // stack: transfer_eth_status, retdest
    %jumpi(process_message_txn_insufficient_balance)
    // stack: retdest

    // If to's code is empty, return.
    %mload_txn_field(@TXN_FIELD_TO) %ext_code_empty
    // stack: code_empty, retdest
    %jumpi(process_message_txn_return)

    // Otherwise, load to's code and execute it in a new context.
    // stack: retdest
    %create_context
    // stack: new_ctx, retdest
    PUSH process_message_txn_code_loaded
    PUSH @SEGMENT_CODE
    DUP3 // new_ctx
    %mload_txn_field(@TXN_FIELD_TO)
    // stack: address, new_ctx, segment, process_message_txn_code_loaded, new_ctx, retdest
    %jump(load_code)

global process_message_txn_insufficient_balance:
    // stack: retdest
    PANIC // TODO

global process_message_txn_return:
    // TODO: Return leftover gas?
    JUMP

global process_message_txn_code_loaded:
    // stack: code_len, new_ctx, retdest
    POP
    // stack: new_ctx, retdest

    // Store the address in metadata.
    %mload_txn_field(@TXN_FIELD_TO)
    PUSH @CTX_METADATA_ADDRESS
    PUSH @SEGMENT_CONTEXT_METADATA
    DUP4 // new_ctx
    MSTORE_GENERAL
    // stack: new_ctx, retdest

    // Store the caller in metadata.
    %mload_txn_field(@TXN_FIELD_ORIGIN)
    PUSH @CTX_METADATA_CALLER
    PUSH @SEGMENT_CONTEXT_METADATA
    DUP4 // new_ctx
    MSTORE_GENERAL
    // stack: new_ctx, retdest

    // Store the call value field in metadata.
    %mload_txn_field(@TXN_FIELD_VALUE)
    PUSH @CTX_METADATA_CALL_VALUE
    PUSH @SEGMENT_CONTEXT_METADATA
    DUP4 // new_ctx
    MSTORE_GENERAL
    // stack: new_ctx, retdest

    // No need to write @CTX_METADATA_STATIC, because it's 0 which is the default.

    // Store parent context in metadata.
    GET_CONTEXT
    PUSH @CTX_METADATA_PARENT_CONTEXT
    PUSH @SEGMENT_CONTEXT_METADATA
    DUP4 // new_ctx
    MSTORE_GENERAL
    // stack: new_ctx, retdest

    // Store parent PC = process_message_txn_after_call.
    PUSH process_message_txn_after_call
    PUSH @CTX_METADATA_PARENT_PC
    PUSH @SEGMENT_CONTEXT_METADATA
    DUP4 // new_ctx
    MSTORE_GENERAL
    // stack: new_ctx, retdest

    // TODO: Populate CALLDATA

    // TODO: Save parent gas and set child gas

    // Now, switch to the new context and go to usermode with PC=0.
    SET_CONTEXT
    // stack: retdest
    PUSH 0 // jump dest
    EXIT_KERNEL

global process_message_txn_after_call:
    // stack: success, retdest
    // TODO: Return leftover gas? Or handled by termination instructions?
    POP // Pop success for now. Will go into the reciept when we support that.
    JUMP
