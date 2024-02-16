// struct CodeChange { address, prev_codehash, prev_code_length }

%macro journal_add_code_change
    %journal_add_3(@JOURNAL_ENTRY_CODE_CHANGE)
%endmacro

global revert_code_change:
    // stack: entry_ptr, ptr, retdest
    POP
    %journal_load_3
    %stack (address, prev_codehash, prev_code_length) -> (address, prev_codehash, address, prev_code_length)
    %key_code %smt_insert_state
    %key_code_length %smt_insert_state
    // stack: retdest
    JUMP
