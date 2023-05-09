// struct AccountDestroyed { address, target, was_destroyed, prev_balance }

%macro journal_add_account_destroyed
    %journal_add_4(@JOURNAL_ENTRY_ACCOUNT_DESTROYED)
%endmacro

global revert_account_destroyed:
    // stack: entry_type, ptr, retdest
    // TODO
    PANIC
