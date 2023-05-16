// struct CodeChange { address, prev_codehash }

%macro journal_add_code_change
    %journal_add_2(@JOURNAL_ENTRY_CODE_CHANGE)
%endmacro

global revert_code_change:
    // stack: entry_ptr, ptr, retdest
    POP
    %journal_load_2
    // stack: address, prev_codehash, retdest
    %mpt_read_state_trie
    // stack: account_ptr, prev_codehash, retdest
    %add_const(3)
    // stack: codehash_ptr, prev_codehash, retdest
    %mstore_trie_data
    // stack: retdest
    JUMP
