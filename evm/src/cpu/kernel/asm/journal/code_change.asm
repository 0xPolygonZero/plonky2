%macro journal_add_code_change
    %journal_add_2(@JOURNAL_ENTRY_CODE_CHANGE)
%endmacro

global revert_code_change:
    // stack: entry_type, ptr, retdest
