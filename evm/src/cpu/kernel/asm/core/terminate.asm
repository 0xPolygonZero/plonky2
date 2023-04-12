// Handlers for operations which terminate the current context, namely STOP,
// RETURN, SELFDESTRUCT, REVERT, and exceptions such as stack underflow.

global sys_stop:
    // TODO: Set parent context's CTX_METADATA_RETURNDATA_SIZE to 0.
    // TODO: Refund unused gas to parent.
    PUSH 1 // success
    %jump(terminate_common)

global sys_return:
    // TODO: Set parent context's CTX_METADATA_RETURNDATA_SIZE.
    // TODO: Copy returned memory to parent context's RETURNDATA (but not if we're returning from a constructor?)
    // TODO: Copy returned memory to parent context's memory (as specified in their call instruction)
    // TODO: Refund unused gas to parent.
    PUSH 1 // success
    %jump(terminate_common)

global sys_selfdestruct:
    %consume_gas_const(@GAS_SELFDESTRUCT)
    // TODO: Destroy account.
    // TODO: Refund unused gas to parent.
    PUSH 1 // success
    %jump(terminate_common)

global sys_revert:
    // TODO: Refund unused gas to parent.
    // TODO: Revert state changes.
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
    // TODO: Revert state changes.
    PUSH 0 // success
    %jump(terminate_common)

terminate_common:
    // stack: success
    // We want to move the success flag from our (child) context's stack to the
    // parent context's stack. We will write it to memory, specifically
    // SEGMENT_KERNEL_GENERAL[0], then load it after the context switch.
    PUSH 0
    // stack: 0, success
    %mstore_kernel_general
    // stack: (empty)

    // Similarly, we write the parent PC to SEGMENT_KERNEL_GENERAL[1] so that
    // we can later read it after switching to the parent context.
    %mload_context_metadata(@CTX_METADATA_PARENT_PC)
    PUSH 1
    %mstore_kernel(@SEGMENT_KERNEL_GENERAL)
    // stack: (empty)

    // Go back to the parent context.
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    SET_CONTEXT
    // stack: (empty)

    // Load the success flag and parent PC that we stored in SEGMENT_KERNEL_GENERAL.
    PUSH 0 %mload_kernel_general
    PUSH 1 %mload_kernel_general

    // stack: parent_pc, success
    JUMP
