global sys_balance:
    // stack: kexit_info, address
    SWAP1 %u256_to_addr
    // stack: address, kexit_info
    DUP1 %insert_accessed_addresses
    // stack: cold_access, address, kexit_info
    PUSH @GAS_COLDACCOUNTACCESS_MINUS_WARMACCESS
    MUL
    PUSH @GAS_WARMACCESS
    ADD
    %stack (gas, address, kexit_info) -> (gas, kexit_info, address)
    %charge_gas
    // stack: kexit_info, address

    SWAP1
    // stack: address, kexit_info
    %balance
    // stack: balance, kexit_info
    SWAP1
    EXIT_KERNEL

%macro balance
    %stack (address) -> (address, %%after)
    %jump(balance)
%%after:
%endmacro

global balance:
    // stack: address, retdest
    %mpt_read_state_trie
    // stack: account_ptr, retdest
    DUP1 ISZERO %jumpi(retzero) // If the account pointer is null, return 0.
    %add_const(1)
    // stack: balance_ptr, retdest
    %mload_trie_data
    // stack: balance, retdest
    SWAP1 JUMP

retzero:
    %stack (account_ptr, retdest) -> (retdest, 0)
    JUMP

global sys_selfbalance:
    // stack: kexit_info
    %charge_gas_const(@GAS_LOW)
    %selfbalance
    // stack: balance, kexit_info
    SWAP1
    EXIT_KERNEL

%macro selfbalance
    PUSH %%after
    %address
    %jump(balance)
%%after:
%endmacro
