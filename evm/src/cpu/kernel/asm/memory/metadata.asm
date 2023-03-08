// Load the given global metadata field from memory.
%macro mload_global_metadata(field)
    // stack: (empty)
    PUSH $field
    // stack: offset
    %mload_kernel(@SEGMENT_GLOBAL_METADATA)
    // stack: value
%endmacro

// Store the given global metadata field to memory.
%macro mstore_global_metadata(field)
    // stack: value
    PUSH $field
    // stack: offset, value
    %mstore_kernel(@SEGMENT_GLOBAL_METADATA)
    // stack: (empty)
%endmacro

// Load the given context metadata field from memory.
%macro mload_context_metadata(field)
    // stack: (empty)
    PUSH $field
    // stack: offset
    %mload_current(@SEGMENT_CONTEXT_METADATA)
    // stack: value
%endmacro

// Store the given context metadata field to memory.
%macro mstore_context_metadata(field)
    // stack: value
    PUSH $field
    // stack: offset, value
    %mstore_current(@SEGMENT_CONTEXT_METADATA)
    // stack: (empty)
%endmacro

%macro address
    %mload_context_metadata(@CTX_METADATA_ADDRESS)
%endmacro

global sys_address:
    // stack: kexit_info
    %address
    // stack: address, kexit_info
    SWAP1
    EXIT_KERNEL

%macro caller
    %mload_context_metadata(@CTX_METADATA_CALLER)
%endmacro

global sys_caller:
    // stack: kexit_info
    %caller
    // stack: caller, kexit_info
    SWAP1
    EXIT_KERNEL

%macro callvalue
    %mload_context_metadata(@CTX_METADATA_CALL_VALUE)
%endmacro

%macro codesize
    %mload_context_metadata(@CTX_METADATA_CODE_SIZE)
%endmacro

global sys_codesize:
    // stack: kexit_info
    %codesize
    // stack: codesize, kexit_info
    SWAP1
    EXIT_KERNEL

global sys_callvalue:
    // stack: kexit_info
    %callvalue
    // stack: callvalue, kexit_info
    SWAP1
    EXIT_KERNEL

%macro msize
    %mload_context_metadata(@CTX_METADATA_MSIZE)
%endmacro

global sys_msize:
    // stack: kexit_info
    %msize
    // stack: msize, kexit_info
    SWAP1
    EXIT_KERNEL

%macro update_msize
    // stack: offset
    %add_const(32)
    // stack: 32 + offset
    %div_const(32)
    // stack: (offset+32)/32 = ceil_div_usize(offset+1, 32)
    %mul_const(32)
    // stack: ceil_div_usize(offset+1, 32) * 32
    %msize
    // stack: current_msize, ceil_div_usize(offset+1, 32) * 32
    %max
    // stack: new_msize
    %mstore_context_metadata(@CTX_METADATA_MSIZE)
%endmacro
