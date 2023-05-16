// struct AccountDestroyed { address, target, prev_balance }

%macro journal_add_account_destroyed
    %journal_add_3(@JOURNAL_ENTRY_ACCOUNT_DESTROYED)
%endmacro

global revert_account_destroyed:
    // stack: entry_type, ptr, retdest
    POP
    %journal_load_3
    // stack: address, target, prev_balance, retdest
    PUSH revert_account_destroyed_contd DUP2
    %jump(remove_selfdestruct_list)
revert_account_destroyed_contd:
    // stack: address, target, prev_balance, retdest
    SWAP1
    // Remove `prev_balance` from `target`'s balance.
    // stack: target, address, prev_balance, retdest
    %mpt_read_state_trie
    %add_const(1)
    // stack: target_balance_ptr, address, prev_balance, retdest
    DUP3
    DUP2 %mload_trie_data
    // stack: target_balance, prev_balance, target_balance_ptr, address, prev_balance, retdest
    SUB SWAP1 %mstore_trie_data
    // Set `address`'s balance to `prev_balance`.
    // stack: address, prev_balance, retdest
    %mpt_read_state_trie
    %add_const(1)
    %mstore_trie_data
    JUMP

