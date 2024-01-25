// struct AccountLoaded { address }

%macro journal_add_account_loaded
    %journal_add_1(@JOURNAL_ENTRY_ACCOUNT_LOADED)
%endmacro

global revert_account_loaded:
    // stack: entry_type, ptr, retdest
    POP
    %journal_load_1
    // stack: address, retdest
    DUP1 %eq_const(@RIP160) %jumpi(ripemd)
    %jump(remove_accessed_addresses)

// The address 0x3 shouldn't become unloaded.
// See https://github.com/ethereum/EIPs/issues/716.
ripemd:
    // stack: address, retdest
    POP JUMP
