// struct Refund { amount }

%macro journal_refund
    %journal_add_1(@JOURNAL_ENTRY_REFUND)
%endmacro

global revert_refund:
    // stack: entry_type, ptr, retdest
    POP
    %journal_load_1
    // stack: amount, retdest
    %mload_global_metadata(@GLOBAL_METADATA_REFUND_COUNTER)
    SUB
    %mstore_global_metadata(@GLOBAL_METADATA_REFUND_COUNTER)
    JUMP
