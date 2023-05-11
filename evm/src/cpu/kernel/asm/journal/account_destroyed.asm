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
    SWAP1 %transfer_eth %jumpi(panic)
    JUMP

