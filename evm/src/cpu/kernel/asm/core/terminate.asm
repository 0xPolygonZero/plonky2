// Handlers for operations which terminate the current context, namely STOP,
// RETURN, SELFDESTRUCT, REVERT, and exceptions such as stack underflow.

global sys_stop:
    // stack: kexit_info
    %leftover_gas
    // stack: leftover_gas
    // TODO: Set parent context's CTX_METADATA_RETURNDATA_SIZE to 0.
    PUSH 1 // success
    %jump(terminate_common)

global sys_return:
    // stack: kexit_info
    %leftover_gas
    // stack: leftover_gas
    // TODO: Set parent context's CTX_METADATA_RETURNDATA_SIZE.
    // TODO: Copy returned memory to parent context's RETURNDATA.
    PUSH 1 // success
    %jump(terminate_common)

global sys_selfdestruct:
    // stack: kexit_info
    %consume_gas_const(@GAS_SELFDESTRUCT)
    %leftover_gas
    // stack: leftover_gas
    // TODO: Destroy account.
    PUSH 1 // success
    %jump(terminate_common)

global sys_revert:
    // stack: kexit_info
    %leftover_gas
    // stack: leftover_gas
    // TODO: Revert state changes.
    // TODO: Set parent context's CTX_METADATA_RETURNDATA_SIZE.
    // TODO: Copy returned memory to parent context's RETURNDATA.
    PUSH 0 // success
    %jump(terminate_common)

// The execution is in an exceptional halt-ing state if
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
    // TODO: Set parent context's CTX_METADATA_RETURNDATA_SIZE to 0.
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

%macro leftover_gas
    // stack: kexit_info
    %shr_const(192)
    // stack: gas_used
    %mload_context_metadata(@CTX_METADATA_GAS_LIMIT)
    SUB
    // stack: leftover_gas
%endmacro
