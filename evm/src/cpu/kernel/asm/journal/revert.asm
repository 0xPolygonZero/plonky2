%macro revert
    // stack: journal_size
    %decrement
    %stack (journal_size_m_1) -> (journal_size_m_1, %%after, journal_size_m_1)
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
    DUP1 %eq_const(@JOURNAL_ENTRY_CODE_CHANGE)       %jumpi(revert_code_change)
    DUP1 %eq_const(@JOURNAL_ENTRY_REFUND)            %jumpi(revert_refund)
    DUP1 %eq_const(@JOURNAL_ENTRY_ACCOUNT_CREATED)   %jumpi(revert_account_created)
    DUP1 %eq_const(@JOURNAL_ENTRY_LOG)               %jumpi(revert_log)
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
    %mstore_global_metadata(@GLOBAL_METADATA_JOURNAL_LEN)
    POP JUMP

revert_one_checkpoint:
    // stack: current_checkpoint, retdest
    DUP1 ISZERO %jumpi(first_checkpoint)
    // stack: current_checkpoint, retdest
    %decrement
    // stack: current_checkpoint-1, retdest
    DUP1 %mload_kernel(@SEGMENT_JOURNAL_CHECKPOINTS)
    // stack: target_size, current_checkpoints-1, retdest
    %jump(do_revert)
first_checkpoint:
    // stack: current_checkpoint, retdest
    %decrement
    // stack: current_checkpoint-1, retdest
    PUSH 0
    // stack: target_size, current_checkpoints-1, retdest
do_revert:
    %stack (target_size, current_checkpoints_m_1, retdest) -> (target_size, after_revert, current_checkpoints_m_1, retdest)
    %jump(revert_batch)
after_revert:
    // stack: current_checkpoint-1, retdest
    SWAP1 JUMP


global revert_checkpoint:
    // stack: retdest
    PUSH 1 %mload_context_metadata(@CTX_METADATA_CHECKPOINTS_LEN) SUB
    %mload_current(@SEGMENT_CONTEXT_CHECKPOINTS)
    // stack: target_checkpoint, retdest
    %current_checkpoint
    // stack: current_checkpoint, target_checkpoint, retdest
    DUP2 DUP2 LT %jumpi(panic) // Sanity check that current_cp >= target_cp. This should never happen.
while:
    // stack: current_checkpoint, target_checkpoint, retdest
    DUP2 DUP2 EQ %jumpi(revert_checkpoint_done)
    %stack (current_checkpoint) -> (current_checkpoint, while)
    %jump(revert_one_checkpoint)
revert_checkpoint_done:
    // stack: current_checkpoint, target_checkpoint, retdest
    POP
    %mstore_global_metadata(@GLOBAL_METADATA_CURRENT_CHECKPOINT)
    %pop_checkpoint
    JUMP

%macro revert_checkpoint
    PUSH %%after
    %jump(revert_checkpoint)
%%after:
    // stack: (empty)
%endmacro
