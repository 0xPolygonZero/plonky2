// struct NonceChange { address, prev_nonce }

%macro journal_add_nonce_change
    %journal_add_2(@JOURNAL_ENTRY_NONCE_CHANGE)
%endmacro

global revert_nonce_change:
    // stack: entry_type, ptr, retdest
    POP
    %journal_load_2
    // stack: address, prev_nonce, retdest
    %mpt_read_state_trie
    // stack: nonce_ptr, prev_nonce retdest
    %mstore_trie_data
    // stack: retdest
    JUMP

