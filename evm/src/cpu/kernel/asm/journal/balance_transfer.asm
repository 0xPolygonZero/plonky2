// struct BalanceTransfer { from, to, balance }

%macro journal_add_balance_transfer
    // stack: from, to, balance
    DUP3 ISZERO %jumpi(%%zero)
    %journal_add_3(@JOURNAL_ENTRY_BALANCE_TRANSFER)
    %jump(%%after)
%%zero:
    // stack: from, to, balance
    %pop3
%%after:
    // stack: (empty)
%endmacro

global revert_balance_transfer:
    // stack: entry_type, ptr, retdest
    POP
    %journal_load_3
    // stack: from, to, balance, retdest
    SWAP1
    // stack: to, from, balance, retdest
    %transfer_eth
    %jumpi(panic) // This should never happen.
    JUMP
