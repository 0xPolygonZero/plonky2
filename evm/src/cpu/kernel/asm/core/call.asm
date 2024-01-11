// Handlers for call-like operations, namely CALL, CALLCODE, STATICCALL and DELEGATECALL.
// Reminder: All context metadata hardcoded offsets are already scaled by `Segment::ContextMetadata`.

// Creates a new sub context and executes the code of the given account.
global sys_call:
    // Check that the value is zero if the context is static.
    // stack: kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size
    DUP4 ISZERO %not_bit
    // stack: value≠0, kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size
    %mload_context_metadata(@CTX_METADATA_STATIC)
    // stack: is_static, value≠0, kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size
    MUL // Cheaper than AND
    %jumpi(fault_exception)

    %stack (kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size) ->
        (args_size, args_offset, kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size)
    %checked_mem_expansion
    %stack (kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size) ->
        (ret_size, ret_offset, kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size)
    %checked_mem_expansion

    SWAP2
    // stack: address, gas, kexit_info, value, args_offset, args_size, ret_offset, ret_size
    %u256_to_addr // Truncate to 160 bits
    DUP1 %insert_accessed_addresses

    %call_charge_gas(1, 1)
    %check_depth

    %checkpoint // Checkpoint
    DUP3 %insert_touched_addresses

    %create_context
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size

    %stack (new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size) ->
          (new_ctx, args_offset, args_size, new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size)
    %copy_mem_to_calldata
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    DUP5 DUP5 %address %transfer_eth %jumpi(call_insufficient_balance)
    DUP5 DUP5 %address %journal_add_balance_transfer
    DUP3 %set_new_ctx_gas_limit
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    DUP4
    // stack: address, new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    %handle_precompiles
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    %set_new_ctx_parent_pc(after_call_instruction)
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size

    // Each line in the block below does not change the stack.
    %set_static
    DUP4 %set_new_ctx_addr
    %address %set_new_ctx_caller
    DUP5 %set_new_ctx_value
    DUP4 %set_new_ctx_code

    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    %stack (new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size)
        -> (new_ctx, kexit_info, ret_offset, ret_size)
    %enter_new_ctx

// Creates a new sub context as if calling itself, but with the code of the
// given account. In particular the storage remains the same.
global sys_callcode:

    // stack: kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size
    %stack (kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size) ->
        (args_size, args_offset, kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size)
    %checked_mem_expansion
    %stack (kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size) ->
        (ret_size, ret_offset, kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size)
    %checked_mem_expansion

    SWAP2
    // stack: address, gas, kexit_info, value, args_offset, args_size, ret_offset, ret_size
    %u256_to_addr // Truncate to 160 bits
    DUP1 %insert_accessed_addresses

    %call_charge_gas(1, 0)
    %check_depth

    %checkpoint // Checkpoint
    %address %insert_touched_addresses

    // stack: kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    %create_context
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size

    %stack (new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size) ->
          (new_ctx, args_offset, args_size, new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size)
    %copy_mem_to_calldata
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    DUP5 %address %address %transfer_eth %jumpi(call_insufficient_balance)
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    DUP3 %set_new_ctx_gas_limit
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    DUP4
    // stack: address, new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    %handle_precompiles
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    %set_new_ctx_parent_pc(after_call_instruction)

    // Each line in the block below does not change the stack.
    %set_static
    %address %set_new_ctx_addr
    %address %set_new_ctx_caller
    DUP5 %set_new_ctx_value
    DUP4 %set_new_ctx_code


    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    %stack (new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size)
        -> (new_ctx, kexit_info, ret_offset, ret_size)
    %enter_new_ctx

// Creates a new sub context and executes the code of the given account.
// Equivalent to CALL, except that it does not allow any state modifying
// instructions or sending ETH in the sub context. The disallowed instructions
// are CREATE, CREATE2, LOG0, LOG1, LOG2, LOG3, LOG4, SSTORE, SELFDESTRUCT and
// CALL if the value sent is not 0.
global sys_staticcall:
    // stack: kexit_info, gas, address, args_offset, args_size, ret_offset, ret_size
    %stack (kexit_info, gas, address, args_offset, args_size, ret_offset, ret_size) ->
        (args_size, args_offset, kexit_info, gas, address, args_offset, args_size, ret_offset, ret_size)
    %checked_mem_expansion
    %stack (kexit_info, gas, address, args_offset, args_size, ret_offset, ret_size) ->
        (ret_size, ret_offset, kexit_info, gas, address, args_offset, args_size, ret_offset, ret_size)
    %checked_mem_expansion

    SWAP2
    // stack: address, gas, kexit_info, args_offset, args_size, ret_offset, ret_size
    %u256_to_addr // Truncate to 160 bits
    DUP1 %insert_accessed_addresses

    // Add a value of 0 to the stack. Slightly inefficient but that way we can reuse %call_charge_gas.
    %stack (cold_access, address, gas, kexit_info) -> (cold_access, address, gas, kexit_info, 0)
    %call_charge_gas(0, 1)
    %check_depth

    %checkpoint // Checkpoint
    DUP3 %insert_touched_addresses

    // stack: kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    %create_context
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size

    %stack (new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size) ->
          (new_ctx, args_offset, args_size, new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size)
    %copy_mem_to_calldata
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    DUP3 %set_new_ctx_gas_limit
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    DUP4
    // stack: address, new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    %handle_precompiles
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    %set_new_ctx_parent_pc(after_call_instruction)
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size

    // Each line in the block below does not change the stack.
    %set_static_true
    DUP4 %set_new_ctx_addr
    %address %set_new_ctx_caller
    PUSH 0 %set_new_ctx_value
    DUP4 %set_new_ctx_code


    %stack (new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size)
        -> (new_ctx, kexit_info, ret_offset, ret_size)
    %enter_new_ctx

// Creates a new sub context as if calling itself, but with the code of the
// given account. In particular the storage, the current sender and the current
// value remain the same.
global sys_delegatecall:

    // stack: kexit_info, gas, address, args_offset, args_size, ret_offset, ret_size
    %stack (kexit_info, gas, address, args_offset, args_size, ret_offset, ret_size) ->
        (args_size, args_offset, kexit_info, gas, address, args_offset, args_size, ret_offset, ret_size)
    %checked_mem_expansion
    %stack (kexit_info, gas, address, args_offset, args_size, ret_offset, ret_size) ->
        (ret_size, ret_offset, kexit_info, gas, address, args_offset, args_size, ret_offset, ret_size)
    %checked_mem_expansion

    SWAP2
    // stack: address, gas, kexit_info, args_offset, args_size, ret_offset, ret_size
    %u256_to_addr // Truncate to 160 bits
    DUP1 %insert_accessed_addresses

    // Add a value of 0 to the stack. Slightly inefficient but that way we can reuse %call_charge_gas.
    %stack (cold_access, address, gas, kexit_info) -> (cold_access, address, gas, kexit_info, 0)
    %call_charge_gas(0, 0)
    %check_depth

    %checkpoint // Checkpoint
    %address %insert_touched_addresses

    // stack: kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    %create_context
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size

    %stack (new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size) ->
          (new_ctx, args_offset, args_size, new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size)
    %copy_mem_to_calldata
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    DUP3 %set_new_ctx_gas_limit
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    DUP4
    // stack: address, new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    %handle_precompiles
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    %set_new_ctx_parent_pc(after_call_instruction)
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size

    // Each line in the block below does not change the stack.
    %set_static
    %address %set_new_ctx_addr
    %caller %set_new_ctx_caller
    %callvalue %set_new_ctx_value
    %set_new_ctx_parent_pc(after_call_instruction)
    DUP4 %set_new_ctx_code

    %stack (new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size)
        -> (new_ctx, kexit_info, ret_offset, ret_size)
    %enter_new_ctx

// We go here after any CALL type instruction (but not after the special call by the transaction originator).
global after_call_instruction:
    // stack: success, leftover_gas, new_ctx, kexit_info, ret_offset, ret_size
    DUP1 ISZERO %jumpi(after_call_instruction_failed)
    %pop_checkpoint
after_call_instruction_contd:
    SWAP3
    // stack: kexit_info, leftover_gas, new_ctx, success, ret_offset, ret_size
    // Add the leftover gas into the appropriate bits of kexit_info.
    SWAP1 %shl_const(192) SWAP1 SUB
    // stack: kexit_info, new_ctx, success, ret_offset, ret_size

    // The callee's terminal instruction will have populated RETURNDATA.
    %copy_returndata_to_mem
    EXIT_KERNEL

after_call_instruction_failed:
    // stack: success, leftover_gas, new_ctx, kexit_info, ret_offset, ret_size
    %revert_checkpoint
    %jump(after_call_instruction_contd)

call_insufficient_balance:
    %stack (new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size) ->
        (callgas, kexit_info, 0)
    %shl_const(192) SWAP1 SUB
    // stack: kexit_info', 0
    %mstore_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 0)
    EXIT_KERNEL

%macro check_depth
    %call_depth
    %gt_const(@CALL_STACK_LIMIT)
    %jumpi(call_too_deep)
%endmacro

call_too_deep:
    %stack (kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size) ->
        (callgas, kexit_info, 0)
    %shl_const(192) SWAP1 SUB
    // stack: kexit_info', 0
    %mstore_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 0)
    EXIT_KERNEL

// Set @CTX_METADATA_STATIC to 1. Note that there is no corresponding set_static_false routine
// because it will already be 0 by default.
%macro set_static_true
    // stack: new_ctx
    DUP1
    %build_address_with_ctx_no_segment(@CTX_METADATA_STATIC)
    PUSH 1
    // stack: 1, addr, new_ctx
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

// Set @CTX_METADATA_STATIC of the next context to the current value.
%macro set_static
    // stack: new_ctx
    DUP1
    %build_address_with_ctx_no_segment(@CTX_METADATA_STATIC)
    %mload_context_metadata(@CTX_METADATA_STATIC)
    // stack: is_static, addr, new_ctx
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro set_new_ctx_addr
    // stack: called_addr, new_ctx
    DUP2
    %build_address_with_ctx_no_segment(@CTX_METADATA_ADDRESS)
    SWAP1
    // stack: called_addr, addr, new_ctx
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro set_new_ctx_caller
    // stack: sender, new_ctx
    DUP2
    %build_address_with_ctx_no_segment(@CTX_METADATA_CALLER)
    SWAP1
    // stack: sender, addr, new_ctx
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro set_new_ctx_value
    // stack: value, new_ctx
    DUP2
    %build_address_with_ctx_no_segment(@CTX_METADATA_CALL_VALUE)
    SWAP1
    // stack: value, addr, new_ctx
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro set_new_ctx_code_size
    // stack: code_size, new_ctx
    DUP2
    %build_address_with_ctx_no_segment(@CTX_METADATA_CODE_SIZE)
    SWAP1
    // stack: code_size, addr, new_ctx
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro set_new_ctx_calldata_size
    // stack: calldata_size, new_ctx
    DUP2
    %build_address_with_ctx_no_segment(@CTX_METADATA_CALLDATA_SIZE)
    SWAP1
    // stack: calldata_size, addr, new_ctx
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro set_new_ctx_gas_limit
    // stack: gas_limit, new_ctx
    DUP2
    %build_address_with_ctx_no_segment(@CTX_METADATA_GAS_LIMIT)
    SWAP1
    // stack: gas_limit, addr, new_ctx
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro set_new_ctx_parent_ctx
    // stack: new_ctx
    DUP1
    %build_address_with_ctx_no_segment(@CTX_METADATA_PARENT_CONTEXT)
    GET_CONTEXT
    // stack: ctx, addr, new_ctx
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro set_new_ctx_parent_pc(label)
    // stack: new_ctx
    DUP1
    %build_address_with_ctx_no_segment(@CTX_METADATA_PARENT_PC)
    PUSH $label
    // stack: label, addr, new_ctx
    MSTORE_GENERAL
    // stack: new_ctx
%endmacro

%macro set_new_ctx_code
    %stack (address, new_ctx) -> (address, new_ctx, %%after, new_ctx)
    %jump(load_code_padded)
%%after:
    %set_new_ctx_code_size
    // stack: new_ctx
%endmacro

%macro enter_new_ctx
    // stack: new_ctx
    // Switch to the new context and go to usermode with PC=0.
    DUP1 // new_ctx
    SET_CONTEXT
    %checkpoint // Checkpoint
    %increment_call_depth
    // Perform jumpdest analyis
    %mload_context_metadata(@CTX_METADATA_CODE_SIZE)
    GET_CONTEXT
    // stack: ctx, code_size, retdest
    %jumpdest_analysis
    PUSH 0 // jump dest
    EXIT_KERNEL
    // (Old context) stack: new_ctx
%endmacro

%macro copy_mem_to_calldata
    // stack: new_ctx, args_offset, args_size
    GET_CONTEXT
    %stack(ctx, new_ctx, args_offset, args_size) -> (ctx, @SEGMENT_MAIN_MEMORY, args_offset, args_size, %%after, new_ctx, args_size)
    %build_address
    // stack: SRC, args_size, %%after, new_ctx, args_size
    DUP4
    %build_address_with_ctx_no_offset(@SEGMENT_CALLDATA)
    // stack: DST, SRC, args_size, %%after, new_ctx, args_size
    %jump(memcpy_bytes)
%%after:
    // stack: new_ctx, args_size
    %build_address_with_ctx_no_segment(@CTX_METADATA_CALLDATA_SIZE)
    // stack: addr, args_size
    SWAP1
    MSTORE_GENERAL
    // stack: (empty)
%endmacro

%macro copy_returndata_to_mem
    // stack: kexit_info, new_ctx, success, ret_offset, ret_size
    SWAP4
    %returndatasize
    // stack: returndata_size, ret_size, new_ctx, success, ret_offset, kexit_info
    %min
    GET_CONTEXT
    %stack (ctx, n, new_ctx, success, ret_offset, kexit_info) -> (ctx, @SEGMENT_RETURNDATA, @SEGMENT_MAIN_MEMORY, ret_offset, ctx, n, %%after, kexit_info, success)
    %build_address_no_offset
    // stack: SRC, @SEGMENT_MAIN_MEMORY, ret_offset, ctx, n, %%after, kexit_info, success
    SWAP3
    %build_address
    // stack: DST, SRC, n, %%after, kexit_info, success
    %jump(memcpy_bytes)
%%after:
%endmacro

// Checked memory expansion.
%macro checked_mem_expansion
    // stack: size, offset, kexit_info
    DUP1 ISZERO %jumpi(%%zero)
    %add_or_fault
    // stack: expanded_num_bytes, kexit_info
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes
    %jump(%%after)
%%zero:
    %pop2
%%after:
%endmacro
