// struct AccountTouched { address }

%macro journal_add_account_touched
    %journal_add_1(@JOURNAL_ENTRY_ACCOUNT_TOUCHED)
%endmacro

global revert_account_touched:
    // stack: entry_type, ptr, retdest
    POP
    %journal_load_1
    // stack: address, retdest
    DUP1 %eq_const(@RIP160) %jumpi(ripemd)
    %jump(remove_touched_addresses)

// The address 0x3 shouldn't become untouched.
// See https://github.com/ethereum/EIPs/issues/716.
ripemd:
    // stack: address, retdest
    POP JUMP
