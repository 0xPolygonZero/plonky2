// struct AccountTouched { address }

%macro journal_add_account_touched
    %journal_add_1(@JOURNAL_ENTRY_ACCOUNT_TOUCHED)
%endmacro

// Note: We don't need to remove touched addresses. In fact doing so leads to bugs because of the way we load accounts in the MPT.
global revert_account_touched:
    // stack: entry_type, ptr, retdest
    %pop2 JUMP
