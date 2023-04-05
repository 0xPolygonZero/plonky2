// Handlers for operations which terminate the current context, namely STOP,
// RETURN, SELFDESTRUCT, REVERT, and exceptions such as stack underflow.

// Set parent context's CTX_METADATA_RETURNDATA_SIZE to 0.
%macro set_parent_returndata_size_zero
    // stack: (empty)
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    %stack (parent_ctx) ->
        (parent_ctx, @SEGMENT_CONTEXT_METADATA, @CTX_METADATA_RETURNDATA_SIZE, 0)
    MSTORE_GENERAL
    // stack: (empty)
%endmacro


global sys_stop:
    // stack: kexit_info

    %set_parent_returndata_size_zero

    %leftover_gas
    // stack: leftover_gas
    PUSH 1 // success
    %jump(terminate_common)

global sys_return:
    // stack: kexit_info, offset, size
    %stack (kexit_info, offset, size) -> (offset, size, kexit_info, offset, size)
    ADD // TODO: Check for overflow?
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes

    // Load the parent's context.
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)

    // Store the return data size in the parent context's metadata.
    %stack (parent_ctx, kexit_info, offset, size) ->
        (parent_ctx, @SEGMENT_CONTEXT_METADATA, @CTX_METADATA_RETURNDATA_SIZE, size, offset, size, parent_ctx, kexit_info)
    MSTORE_GENERAL
    // stack: offset, size, parent_ctx, kexit_info

    // Store the return data in the parent context's returndata segment.
    GET_CONTEXT
    %stack (ctx, offset, size, parent_ctx, kexit_info) ->
        (
        parent_ctx, @SEGMENT_RETURNDATA, 0, // DST
        ctx, @SEGMENT_MAIN_MEMORY, offset,  // SRC
        size, sys_return_finish, kexit_info // count, retdest, ...
        )
    %jump(memcpy)

sys_return_finish:
    // stack: kexit_info
    %leftover_gas
    // stack: leftover_gas
    PUSH 1 // success
    %jump(terminate_common)

global sys_selfdestruct:
    // stack: kexit_info, recipient
    SWAP1 %u256_to_addr
    %address DUP1 %balance

    // Insert recipient into the accessed addresses list.
    // stack: balance, address, recipient, kexit_info
    DUP3 %insert_accessed_addresses

    %set_parent_returndata_size_zero

    // Compute gas.
    // stack: cold_access, balance, address, recipient, kexit_info
    %mul_const(@GAS_COLDACCOUNTACCESS)
    DUP2
    // stack: balance, gas_coldaccess, balance, address, recipient, kexit_info
    ISZERO %not_bit
    // stack: balance!=0, gas_coldaccess, balance, address, recipient, kexit_info
    DUP5 %is_dead MUL %mul_const(@GAS_NEWACCOUNT)
    // stack: gas_newaccount, gas_coldaccess, balance, address, recipient, kexit_info
    ADD %add_const(@GAS_SELFDESTRUCT)
    %stack (gas, balance, address, recipient, kexit_info) -> (gas, kexit_info, balance, address, recipient)
    %charge_gas
    %stack (kexit_info, balance, address, recipient) -> (balance, address, recipient, kexit_info)

    // Insert address into the selfdestruct set.
    // stack: balance, address, recipient, kexit_info
    DUP2 %insert_selfdestruct_list

    // Set the balance of the address to 0.
    // stack: balance, address, recipient, kexit_info
    PUSH 0
    // stack: 0, balance, address, recipient, kexit_info
    DUP3 %mpt_read_state_trie
    // stack: account_ptr, 0, balance, address, recipient, kexit_info
    %add_const(1)
    // stack: balance_ptr, 0, balance, address, recipient, kexit_info
    %mstore_trie_data // TODO: This should be a copy-on-write operation.

    // If the recipient is the same as the address, then we're done.
    // Otherwise, send the balance to the recipient.
    %stack (balance, address, recipient, kexit_info) -> (address, recipient, recipient, balance, kexit_info)
    EQ %jumpi(sys_selfdestruct_same_addr)
    // stack: recipient, balance, kexit_info
    %add_eth

    // stack: kexit_info
    %leftover_gas
    // stack: leftover_gas
    PUSH 1 // success
    %jump(terminate_common)

sys_selfdestruct_same_addr:
    // stack: recipient, balance, kexit_info
    %pop2
    %leftover_gas
    // stack: leftover_gas
    PUSH 1 // success
    %jump(terminate_common)

global sys_revert:
    // stack: kexit_info, offset, size
    %stack (kexit_info, offset, size) -> (offset, size, kexit_info, offset, size)
    ADD // TODO: Check for overflow?
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes

    // Load the parent's context.
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)

    // Store the return data size in the parent context's metadata.
    %stack (parent_ctx, kexit_info, offset, size) ->
        (parent_ctx, @SEGMENT_CONTEXT_METADATA, @CTX_METADATA_RETURNDATA_SIZE, size, offset, size, parent_ctx, kexit_info)
    MSTORE_GENERAL
    // stack: offset, size, parent_ctx, kexit_info

    // Store the return data in the parent context's returndata segment.
    GET_CONTEXT
    %stack (ctx, offset, size, parent_ctx, kexit_info) ->
        (
        parent_ctx, @SEGMENT_RETURNDATA, 0, // DST
        ctx, @SEGMENT_MAIN_MEMORY, offset,  // SRC
        size, sys_revert_finish, kexit_info // count, retdest, ...
        )
    %jump(memcpy)

sys_revert_finish:
    %leftover_gas
    // stack: leftover_gas
    // TODO: Revert state changes.
    PUSH 0 // success
    %jump(terminate_common)

// The execution is in an exceptional halting state if
// - there is insufficient gas
// - the instruction is invalid
// - there are insufficient stack items
// - a JUMP/JUMPI destination is invalid
// - the new stack size would be larger than 1024, or
// - state modification is attempted during a static call
global fault_exception:
    // stack: (empty)
    PUSH 0 // leftover_gas
    // TODO: Revert state changes.
    %set_parent_returndata_size_zero
    PUSH 0 // success
    %jump(terminate_common)

global terminate_common:
    // stack: success, leftover_gas
    // TODO: Panic if we exceeded our gas limit?

    // We want to move the success flag from our (child) context's stack to the
    // parent context's stack. We will write it to memory, specifically
    // SEGMENT_KERNEL_GENERAL[0], then load it after the context switch.
    PUSH 0
    // stack: 0, success, leftover_gas
    %mstore_kernel_general
    // stack: leftover_gas

    // Similarly, we write leftover_gas to SEGMENT_KERNEL_GENERAL[1] so that
    // we can later read it after switching to the parent context.
    PUSH 1
    // stack: 1, leftover_gas
    %mstore_kernel_general
    // stack: (empty)

    // Similarly, we write the parent PC to SEGMENT_KERNEL_GENERAL[2] so that
    // we can later read it after switching to the parent context.
    %mload_context_metadata(@CTX_METADATA_PARENT_PC)
    PUSH 2
    %mstore_kernel(@SEGMENT_KERNEL_GENERAL)
    // stack: (empty)

    // Go back to the parent context.
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    SET_CONTEXT
    // stack: (empty)

    // Load the fields that we stored in SEGMENT_KERNEL_GENERAL.
    PUSH 1 %mload_kernel_general // leftover_gas
    PUSH 0 %mload_kernel_general // success
    PUSH 2 %mload_kernel_general // parent_pc

    // stack: parent_pc, success, leftover_gas
    JUMP
