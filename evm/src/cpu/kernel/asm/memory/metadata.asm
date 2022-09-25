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

%macro sender
    %mload_context_metadata(@CTX_METADATA_CALLER)
%endmacro

%macro callvalue
    %mload_context_metadata(@CTX_METADATA_CALL_VALUE)
%endmacro
