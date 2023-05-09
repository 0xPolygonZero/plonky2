// struct AccountLoaded { address }

%macro journal_add_account_loaded
    %journal_add_1(@JOURNAL_ENTRY_ACCOUNT_LOADED)
%endmacro

global revert_account_loaded:
    // stack: entry_type, ptr, retdest
    POP
    %journal_load_1
    // stack: address, retdest
    %jump(remove_accessed_addresses)