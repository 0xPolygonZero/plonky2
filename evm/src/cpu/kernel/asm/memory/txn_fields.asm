// Load the given normalized transaction field from memory.
%macro mload_txn_field(field)
    // stack: (empty)
    PUSH $field
    // stack: offset
    %mload_kernel(@SEGMENT_NORMALIZED_TXN)
    // stack: value
%endmacro

// Store the given normalized transaction field to memory.
%macro mstore_txn_field(field)
    // stack: value
    PUSH $field
    // stack: offset, value
    %mstore_kernel(@SEGMENT_NORMALIZED_TXN)
    // stack: (empty)
%endmacro
