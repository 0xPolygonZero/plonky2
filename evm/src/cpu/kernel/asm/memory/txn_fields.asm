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

%macro origin
    %mload_txn_field(@TXN_FIELD_ORIGIN)
%endmacro

global sys_origin:
    // stack: kexit_info
    %charge_gas_const(@GAS_BASE)
    // stack: kexit_info
    %origin
    // stack: origin, kexit_info
    SWAP1
    EXIT_KERNEL
