// struct StorageChange { address, slot, prev_value }

%macro journal_add_storage_change
    %journal_add_3(@JOURNAL_ENTRY_STORAGE_CHANGE)
%endmacro

global revert_storage_change:
    // stack: entry_type, ptr, retdest
    POP
    %journal_load_3
    // stack: address, slot, prev_value, retdest
    DUP3 ISZERO %jumpi(delete)
    // stack: address, slot, prev_value, retdest
    %key_storage %smt_insert_state
    // stack: retdest
    JUMP

delete:
    // stack: address, slot, prev_value, retdest
    %key_storage %smt_delete_state
    // stack: prev_value, retdest
    POP JUMP
