%macro revert
    // stack: journal_size
    %decrement
    DUP1
    // stack: journal_size-1, journal_size-1
    PUSH %%after
    SWAP1
    %mload_journal
    // stack: ptr, %%after, journal_size-1
    DUP1 %mload_journal_data
    // stack: entry_type, ptr, %%after, journal_size-1
    DUP1 %eq_const(@JOURNAL_ENTRY_ACCOUNT_LOADED)    %jumpi(revert_account_loaded)
    DUP1 %eq_const(@JOURNAL_ENTRY_ACCOUNT_DESTROYED) %jumpi(revert_account_destroyed)
    DUP1 %eq_const(@JOURNAL_ENTRY_ACCOUNT_TOUCHED)   %jumpi(revert_account_touched)
    DUP1 %eq_const(@JOURNAL_ENTRY_BALANCE_TRANSFER)  %jumpi(revert_balance_transfer)
    DUP1 %eq_const(@JOURNAL_ENTRY_NONCE_CHANGE)      %jumpi(revert_nonce_change)
    DUP1 %eq_const(@JOURNAL_ENTRY_STORAGE_CHANGE)    %jumpi(revert_storage_change)
    DUP1 %eq_const(@JOURNAL_ENTRY_STORAGE_LOADED)    %jumpi(revert_storage_loaded)
         %eq_const(@JOURNAL_ENTRY_CODE_CHANGE)       %jumpi(revert_code_change)
    PANIC // This should never happen.
%%after:
    // stack: journal_size-1
%endmacro

global revert_batch:
    // stack: target_size, retdest
    %journal_size
    // stack: journal_size, target_size, retdest
    DUP2 DUP2 LT %jumpi(panic) // Sanity check to avoid infinite loop.
while_loop:
    // stack: journal_size, target_size, retdest
    DUP2 DUP2 EQ %jumpi(revert_batch_done)
    // stack: journal_size, target_size, retdest
    %revert
    // stack: journal_size-1, target_size, retdest
    %jump(while_loop)

revert_batch_done:
    // stack: journal_size, target_size, retdest
    %pop2 JUMP
