// Load the given normalized transaction field from memory.
%macro mload_txn_field(field)
    // Transaction fields are already scaled by their corresponding segment,
    // effectively making them the direct memory position to read from /
    // write to.

    // stack: (empty)
    PUSH $field
    // stack: addr
    MLOAD_GENERAL
    // stack: value
%endmacro

// Store the given normalized transaction field to memory.
%macro mstore_txn_field(field)
    // Transaction fields are already scaled by their corresponding segment,
    // effectively making them the direct memory position to read from /
    // write to.

    // stack: value
    PUSH $field
    // stack: addr, value
    SWAP1
    MSTORE_GENERAL
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
