%macro withdrawals
    // stack: (empty)
    PUSH %%after
    %jump(withdrawals)
%%after:
    // stack: (empty)
%endmacro

global withdrawals:
    // stack: retdest
    PROVER_INPUT(withdrawal)
    // stack: address, retdest
    PROVER_INPUT(withdrawal)
    // stack: amount, address, retdest
    DUP2 %eq_const(@U256_MAX) %jumpi(withdrawals_end)
    SWAP1
    // stack: address, amount, retdest
    %add_eth
    // stack: retdest
    %jump(withdrawals)

withdrawals_end:
    // stack: amount, address, retdest
    %pop2
    JUMP
