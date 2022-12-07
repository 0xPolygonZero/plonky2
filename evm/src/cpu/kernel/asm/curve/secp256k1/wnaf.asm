global wnaf:
    // stack: segment, n, retdest
    PUSH 0
wnaf_loop:
    %stack (o, segment, n, retdest) -> (n, wnaf_loop_contd, o, segment, retdest)
    %jump(trailing_zeros)
wnaf_loop_contd:
    %stack (n, i, o, segment, retdest) -> (o, i, n, segment, retdest)
    ADD
    %stack (o, n, segment, retdest) -> (n, segment, o, retdest)
    DUP1 %and_const(31) SWAP1
    PUSH 16 DUP3 GT
    // stack: m>16, n, m, segment, o, retdest
    %mul_const(32) ADD
    // stack: n, m, segment, o, retdest
    DUP2 SWAP1 SUB
    %stack (n, m, segment, o, retdest) -> (127, o, m, o, segment, n, retdest)
    SUB
    %stack (i, m, o, segment, n, retdest) -> (0, segment, i, m, o, segment, n, retdest)
    MSTORE_GENERAL
    // stack: o, segment, n, retdest
    DUP3 ISZERO %jumpi(wnaf_end)
    // stack: o, segment, n, retdest
    %jump(wnaf_loop)

wnaf_end:
    // stack: o, segment, n, retdest
    %pop3 JUMP



trailing_zeros:
    // stack: x, retdest
    PUSH 0
trailing_zeros_loop:
    // stack: count, x, retdest
    PUSH 1 DUP3 AND
    // stack: x&1, count, x, retdest
    %jumpi(trailing_zeros_end)
    // stack: count, x, retdest
    %increment SWAP1 PUSH 1 SHR SWAP1
    // stack: count, x, retdest
    %jump(trailing_zeros_loop)
trailing_zeros_end:
    %stack (count, x, retdest) -> (retdest, x, count)
    JUMP
