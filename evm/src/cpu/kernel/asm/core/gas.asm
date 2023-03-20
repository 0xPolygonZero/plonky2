global sys_gas:
    // stack: kexit_info
    %charge_gas_const(@GAS_BASE)
    // stack: kexit_info
    DUP1 %shr_const(192)
    // stack: gas_used, kexit_info
    %ctx_gas_limit
    // stack: gas_limit, gas_used, kexit_info
    SUB
    // stack: gas_remaining, kexit_info
    SWAP1
    EXIT_KERNEL

%macro ctx_gas_limit
    %mload_context_metadata(@CTX_METADATA_GAS_LIMIT)
%endmacro

// Charge gas. Faults if we exceed the limit for the current context.
%macro charge_gas
    // stack: gas, kexit_info
    %shl_const(192)
    ADD
    // stack: kexit_info'
    %ctx_gas_limit
    // stack: gas_limit, kexit_info'
    DUP2 %shr_const(192)
    // stack: gas_used, gas_limit, kexit_info'
    GT
    // stack: out_of_gas, kexit_info'
    %jumpi(fault_exception)
    // stack: kexit_info'
%endmacro

// Charge a constant amount of gas.
%macro charge_gas_const(gas)
    // stack: kexit_info
    PUSH $gas
    // stack: gas, kexit_info
    %charge_gas
    // stack: kexit_info'
%endmacro

// Charge gas and exit kernel code.
%macro charge_gas_and_exit
    // stack: gas, kexit_info
    %charge_gas
    // stack: kexit_info'
    EXIT_KERNEL
%endmacro

global sys_gasprice:
    // stack: kexit_info
    %charge_gas_const(@GAS_BASE)
    // stack: kexit_info
    %mload_txn_field(@TXN_FIELD_COMPUTED_FEE_PER_GAS)
    // stack: gas_price, kexit_info
    SWAP1
    EXIT_KERNEL
