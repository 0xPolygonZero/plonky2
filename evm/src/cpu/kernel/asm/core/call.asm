// Handlers for call-like operations, namely CALL, CALLCODE, STATICCALL and DELEGATECALL.

// Creates a new sub context and executes the code of the given account.
global call:
    // stack: gas, address, value, args_offset, args_size, ret_offset, ret_size, retdest
    %address
    %stack (self, gas, address, value)
           // These are (static, should_transfer_value, value, sender, address, code_addr, gas)
        -> (0, 1, value, self, address, address, gas)
    %jump(call_common)

// Creates a new sub context as if calling itself, but with the code of the
// given account. In particular the storage remains the same.
global call_code:
    // stack: gas, address, value, args_offset, args_size, ret_offset, ret_size, retdest
    %address
    %stack (self, gas, address, value)
           // These are (static, should_transfer_value, value, sender, address, code_addr, gas)
        -> (0, 1, value, self, self, address, gas)
    %jump(call_common)

// Creates a new sub context and executes the code of the given account.
// Equivalent to CALL, except that it does not allow any state modifying
// instructions or sending ETH in the sub context. The disallowed instructions
// are CREATE, CREATE2, LOG0, LOG1, LOG2, LOG3, LOG4, SSTORE, SELFDESTRUCT and
// CALL if the value sent is not 0.
global static_all:
    // stack: gas, address, args_offset, args_size, ret_offset, ret_size, retdest
    %address
    %stack (self, gas, address)
           // These are (static, should_transfer_value, value, sender, address, code_addr, gas)
        -> (1, 0, 0, self, address, address, gas)
    %jump(call_common)

// Creates a new sub context as if calling itself, but with the code of the
// given account. In particular the storage, the current sender and the current
// value remain the same.
global delegate_call:
    // stack: gas, address, args_offset, args_size, ret_offset, ret_size, retdest
    %address
    %sender
    %callvalue
    %stack (self, sender, value, gas, address)
           // These are (static, should_transfer_value, value, sender, address, code_addr, gas)
        -> (0, 0, value, sender, self, address, gas)
    %jump(call_common)

call_common:
    // stack: static, should_transfer_value, value, sender, address, code_addr, gas, args_offset, args_size, ret_offset, ret_size, retdest
    %create_context
    // Store the static flag in metadata.
    %stack (new_ctx, static) -> (new_ctx, @SEGMENT_CONTEXT_METADATA, @CTX_METADATA_STATIC, static, new_ctx)
    MSTORE_GENERAL
    // stack: new_ctx, should_transfer_value, value, sender, address, code_addr, gas, args_offset, args_size, ret_offset, ret_size, retdest

    // Store the address in metadata.
    %stack (new_ctx, should_transfer_value, value, sender, address)
        -> (new_ctx, @SEGMENT_CONTEXT_METADATA, @CTX_METADATA_ADDRESS, address,
            new_ctx, should_transfer_value, value, sender, address)
    MSTORE_GENERAL
    // stack: new_ctx, should_transfer_value, value, sender, address, code_addr, gas, args_offset, args_size, ret_offset, ret_size, retdest

    // Store the caller in metadata.
    %stack (new_ctx, should_transfer_value, value, sender)
        -> (new_ctx, @SEGMENT_CONTEXT_METADATA, @CTX_METADATA_CALLER, sender,
            new_ctx, should_transfer_value, value, sender)
    MSTORE_GENERAL
    // stack: new_ctx, should_transfer_value, value, sender, address, code_addr, gas, args_offset, args_size, ret_offset, ret_size, retdest

    // Store the call value field in metadata.
    %stack (new_ctx, should_transfer_value, value, sender, address) =
        -> (new_ctx, @SEGMENT_CONTEXT_METADATA, @CTX_METADATA_CALL_VALUE, value,
            should_transfer_value, sender, address, value, new_ctx)
    MSTORE_GENERAL
    // stack: should_transfer_value, sender, address, value, new_ctx, code_addr, gas, args_offset, args_size, ret_offset, ret_size, retdest

    %maybe_transfer_eth
    // stack: new_ctx, code_addr, gas, args_offset, args_size, ret_offset, ret_size, retdest

    // Store parent context in metadata.
    GET_CONTEXT
    PUSH @CTX_METADATA_PARENT_CONTEXT
    PUSH @SEGMENT_CONTEXT_METADATA
    DUP4 // new_ctx
    MSTORE_GENERAL
    // stack: new_ctx, code_addr, gas, args_offset, args_size, ret_offset, ret_size, retdest

    // Store parent PC = after_call.
    %stack (new_ctx) -> (new_ctx, @SEGMENT_CONTEXT_METADATA, @CTX_METADATA_PARENT_PC, after_call, new_ctx)
    MSTORE_GENERAL
    // stack: new_ctx, code_addr, gas, args_offset, args_size, ret_offset, ret_size, retdest

    // TODO: Populate CALLDATA
    // TODO: Save parent gas and set child gas
    // TODO: Populate code

    // TODO: Temporary, remove after above steps are done.
    %stack (new_ctx, code_addr, gas, args_offset, args_size) -> (new_ctx)
    // stack: new_ctx, ret_offset, ret_size, retdest

    // Now, switch to the new context and go to usermode with PC=0.
    DUP1 // new_ctx
    SET_CONTEXT
    PUSH 0 // jump dest
    EXIT_KERNEL

after_call:
    // stack: new_ctx, ret_offset, ret_size, retdest
    // TODO: Set RETURNDATA.
    // TODO: Return to caller w/ EXIT_KERNEL.
