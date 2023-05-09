// struct CodeChange { address, prev_codehash }

%macro journal_add_code_change
    %journal_add_2(@JOURNAL_ENTRY_CODE_CHANGE)
%endmacro

global revert_code_change:
    // stack: entry_type, ptr, retdest
    POP
    %journal_load_2
    // stack: address, prev_codehash, retdest
    %jump(set_codehash)
