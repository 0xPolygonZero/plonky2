// struct AccountTouched { address }

%macro journal_add_account_touched
    %journal_add_1(@JOURNAL_ENTRY_ACCOUNT_TOUCHED)
%endmacro

global revert_account_touched:
    // stack: entry_type, ptr, retdest
    POP
    %journal_load_1
    // stack: address, retdest
    %jump(remove_touched_addresses)