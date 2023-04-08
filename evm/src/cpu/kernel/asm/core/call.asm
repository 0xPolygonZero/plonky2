// Handlers for call-like operations, namely CALL, CALLCODE, STATICCALL and DELEGATECALL.

// Creates a new sub context and executes the code of the given account.
global sys_call:
    // stack: kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size
    // TODO: Charge gas.
    SWAP2
    // stack: address, gas, kexit_info, value, args_offset, args_size, ret_offset, ret_size
    %u256_to_addr // Truncate to 160 bits
    DUP1 %insert_accessed_addresses POP // TODO: Use return value in gas calculation.
    SWAP2
    // stack: kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size
    %create_context
    // stack: new_ctx, kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size
    // TODO: Consider call depth

    // Each line in the block below does not change the stack.
    DUP4 %set_new_ctx_addr
    %address %set_new_ctx_caller
    DUP5 %set_new_ctx_value
    DUP5 DUP5 %address %transfer_eth %jumpi(panic) // TODO: Fix this panic.
    %set_new_ctx_parent_ctx
    %set_new_ctx_parent_pc(after_call_instruction)
    DUP3 %set_new_ctx_gas_limit // TODO: This is not correct in most cases. Use C_callgas as in the YP.
    DUP4 %set_new_ctx_code

    %stack (new_ctx, kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size) ->
          (new_ctx, args_offset, args_size, new_ctx, kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size)
    %copy_mem_to_calldata

    // stack: new_ctx, kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size
    %stack (new_ctx, kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size)
        -> (new_ctx, kexit_info, ret_offset, ret_size)
    %enter_new_ctx

// Creates a new sub context as if calling itself, but with the code of the
// given account. In particular the storage remains the same.
global sys_callcode:
    // stack: kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size
    // TODO: Charge gas.
    SWAP2
    // stack: address, gas, kexit_info, value, args_offset, args_size, ret_offset, ret_size
    %u256_to_addr // Truncate to 160 bits
    DUP1 %insert_accessed_addresses POP // TODO: Use return value in gas calculation.
    SWAP2
    // stack: kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size
    %create_context
    // stack: new_ctx, kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size

    // Each line in the block below does not change the stack.
    %address %set_new_ctx_addr
    %address %set_new_ctx_caller
    DUP5 %set_new_ctx_value
    DUP5 DUP5 %address %transfer_eth %jumpi(panic) // TODO: Fix this panic.
    %set_new_ctx_parent_ctx
    %set_new_ctx_parent_pc(after_call_instruction)

    // stack: new_ctx, kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size
    %stack (new_ctx, kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size)
        -> (new_ctx, kexit_info, ret_offset, ret_size)
    %enter_new_ctx

// Creates a new sub context and executes the code of the given account.
// Equivalent to CALL, except that it does not allow any state modifying
// instructions or sending ETH in the sub context. The disallowed instructions
// are CREATE, CREATE2, LOG0, LOG1, LOG2, LOG3, LOG4, SSTORE, SELFDESTRUCT and
// CALL if the value sent is not 0.
global sys_staticcall:
    // stack: kexit_info, gas, address, args_offset, args_size, ret_offset, ret_size
    // TODO: Charge gas.
    SWAP2
    // stack: address, gas, kexit_info, args_offset, args_size, ret_offset, ret_size
    %u256_to_addr // Truncate to 160 bits
    DUP1 %insert_accessed_addresses POP // TODO: Use return value in gas calculation.
    SWAP2
    // stack: kexit_info, gas, address, args_offset, args_size, ret_offset, ret_size
    %create_context
    // stack: new_ctx, kexit_info, gas, address, args_offset, args_size, ret_offset, ret_size

    // Each line in the block below does not change the stack.
    %set_static_true
    DUP4 %set_new_ctx_addr
    %address %set_new_ctx_caller
    PUSH 0 %set_new_ctx_value
    %set_new_ctx_parent_ctx
    %set_new_ctx_parent_pc(after_call_instruction)

    %stack (new_ctx, kexit_info, gas, address, args_offset, args_size, ret_offset, ret_size)
        -> (new_ctx, kexit_info, ret_offset, ret_size)
    %enter_new_ctx

// Creates a new sub context as if calling itself, but with the code of the
// given account. In particular the storage, the current sender and the current
// value remain the same.
global sys_delegatecall:
    // stack: kexit_info, gas, address, args_offset, args_size, ret_offset, ret_size
    // TODO: Charge gas.
    SWAP2
    // stack: address, gas, kexit_info, args_offset, args_size, ret_offset, ret_size
    %u256_to_addr // Truncate to 160 bits
    DUP1 %insert_accessed_addresses POP // TODO: Use return value in gas calculation.
    SWAP2
    // stack: kexit_info, gas, address, args_offset, args_size, ret_offset, ret_size
    %create_context
    // stack: new_ctx, kexit_info, gas, address, args_offset, args_size, ret_offset, ret_size

    // Each line in the block below does not change the stack.
    %address %set_new_ctx_addr
    %caller %set_new_ctx_caller
    %callvalue %set_new_ctx_value
    %set_new_ctx_parent_ctx
    %set_new_ctx_parent_pc(after_call_instruction)

    %stack (new_ctx, kexit_info, gas, address, args_offset, args_size, ret_offset, ret_size)
        -> (new_ctx, kexit_info, ret_offset, ret_size)
    %enter_new_ctx

// We go here after any CALL type instruction (but not after the special call by the transaction originator).
global after_call_instruction:
    // stack: success, leftover_gas, new_ctx, kexit_info, ret_offset, ret_size
    SWAP3
    // stack: kexit_info, leftover_gas, new_ctx, success, ret_offset, ret_size
    // Add the leftover gas into the appropriate bits of kexit_info.
    SWAP1 %shl_const(192) ADD
    // stack: kexit_info, new_ctx, success, ret_offset, ret_size

    // The callee's terminal instruction will have populated RETURNDATA.
    %copy_returndata_to_mem
    EXIT_KERNEL

// Set @CTX_METADATA_STATIC to 1. Note that there is no corresponding set_static_false routine
// because it will already be 0 by default.
%macro set_static_true
    // stack: new_ctx
    %stack (new_ctx) -> (new_ctx, @SEGMENT_CONTEXT_METADATA, @CTX_METADATA_STATIC, 1, new_ctx)
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro set_new_ctx_addr
    // stack: called_addr, new_ctx
    %stack (called_addr, new_ctx)
        -> (new_ctx, @SEGMENT_CONTEXT_METADATA, @CTX_METADATA_ADDRESS, called_addr, new_ctx)
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro set_new_ctx_caller
    // stack: sender, new_ctx
    %stack (sender, new_ctx)
        -> (new_ctx, @SEGMENT_CONTEXT_METADATA, @CTX_METADATA_CALLER, sender, new_ctx)
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro set_new_ctx_value
    // stack: value, new_ctx
    %stack (value, new_ctx)
        -> (new_ctx, @SEGMENT_CONTEXT_METADATA, @CTX_METADATA_CALL_VALUE, value, new_ctx)
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro set_new_ctx_code_size
    // stack: code_size, new_ctx
    %stack (code_size, new_ctx)
        -> (new_ctx, @SEGMENT_CONTEXT_METADATA, @CTX_METADATA_CODE_SIZE, code_size, new_ctx)
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro set_new_ctx_calldata_size
    // stack: calldata_size, new_ctx
    %stack (calldata_size, new_ctx)
        -> (new_ctx, @SEGMENT_CONTEXT_METADATA, @CTX_METADATA_CALLDATA_SIZE, calldata_size, new_ctx)
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro set_new_ctx_gas_limit
    // stack: gas_limit, new_ctx
    %stack (gas_limit, new_ctx)
        -> (new_ctx, @SEGMENT_CONTEXT_METADATA, @CTX_METADATA_GAS_LIMIT, gas_limit, new_ctx)
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro set_new_ctx_parent_ctx
    // stack: new_ctx
    GET_CONTEXT
    PUSH @CTX_METADATA_PARENT_CONTEXT
    PUSH @SEGMENT_CONTEXT_METADATA
    DUP4 // new_ctx
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro set_new_ctx_parent_pc(label)
    // stack: new_ctx
    %stack (new_ctx)
        -> (new_ctx, @SEGMENT_CONTEXT_METADATA, @CTX_METADATA_PARENT_PC, $label, new_ctx)
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro set_new_ctx_code
    %stack (address, new_ctx) -> (address, new_ctx, @SEGMENT_CODE, %%after, new_ctx)
    %jump(load_code)
%%after:
    %stack (code_size, new_ctx)
        -> (new_ctx, @SEGMENT_CONTEXT_METADATA, @CTX_METADATA_CODE_SIZE, code_size, new_ctx)
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro enter_new_ctx
    // stack: new_ctx
    // Switch to the new context and go to usermode with PC=0.
    DUP1 // new_ctx
    SET_CONTEXT
    PUSH 0 // jump dest
    EXIT_KERNEL
    // (Old context) stack: new_ctx
%endmacro

%macro copy_mem_to_calldata
    // stack: new_ctx, args_offset, args_size
    GET_CONTEXT
    %stack (ctx, new_ctx, args_offset, args_size) ->
        (
            new_ctx, @SEGMENT_CALLDATA, 0,          // DST
            ctx, @SEGMENT_MAIN_MEMORY, args_offset, // SRC
            args_size, %%after,                     // count, retdest
            new_ctx, args_size
        )
    %jump(memcpy)
%%after:
    %stack (new_ctx, args_size) ->
        (new_ctx, @SEGMENT_CONTEXT_METADATA, @CTX_METADATA_CALLDATA_SIZE, args_size)
    MSTORE_GENERAL
    // stack: (empty)
%endmacro

%macro copy_returndata_to_mem
    // stack: kexit_info, new_ctx, success, ret_offset, ret_size
    GET_CONTEXT
    %stack (ctx, kexit_info, new_ctx, success, ret_offset, ret_size) ->
        (
            ctx, @SEGMENT_MAIN_MEMORY, ret_offset, // DST
            ctx, @SEGMENT_RETURNDATA, 0,           // SRC
            ret_size, %%after,                     // count, retdest
            kexit_info, success
        )
    %jump(memcpy)
%%after:
%endmacro
