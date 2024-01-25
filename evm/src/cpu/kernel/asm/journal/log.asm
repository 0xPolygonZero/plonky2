// struct Log { logs_data_len, logs_payload_len }

%macro journal_add_log
    %journal_add_2(@JOURNAL_ENTRY_LOG)
%endmacro

global revert_log:
    // stack: entry_type, ptr, retdest
    POP
    // First, reduce the number of logs.
    PUSH 1
    %mload_global_metadata(@GLOBAL_METADATA_LOGS_LEN)
    SUB
    %mstore_global_metadata(@GLOBAL_METADATA_LOGS_LEN)
    // stack: ptr, retdest
    // Second, restore payload length.
    %journal_load_2
    // stack: prev_logs_data_len, prev_payload_len, retdest
    %mstore_global_metadata(@GLOBAL_METADATA_LOGS_DATA_LEN)
    %mstore_global_metadata(@GLOBAL_METADATA_LOGS_PAYLOAD_LEN)
    JUMP
