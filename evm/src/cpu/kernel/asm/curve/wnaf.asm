// wNAF expansion with w=5.
// Stores the reversed expansion of the given scalar in memory at the given segment and offsets 0..130.
// Should be called with scalars of bit length <= 129, which is the case when using GLV.
// Pseudo-code:
// def wnaf(n):
//     ans = [0 for _ in range(130)]
//     o = 0
//     while n != 0:
//         i = n.trailing_zero_bits()
//         o += i
//         n >>= i
//         m = n & 31
//         ans[o] = m
//         if m > 16:
//             ne += 32
//         ne -= m
//     return ans
global wnaf:
    // stack: N, segment, n, retdest (N is the size of the group in which the mul is taking place)
    DUP3 MOD ISZERO %jumpi(wnaf_zero_scalar)
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
    %stack (n, m, segment, o, retdest) -> (129, o, m, o, segment, n, retdest)
    SUB
    // stack:  i, m, o, segment, n, retdest
    DUP4
    GET_CONTEXT
    %build_address
    // stack:  addr, m, o, segment, n, retdest
    SWAP1
    MSTORE_GENERAL
    // stack: o, segment, n, retdest
    DUP3 ISZERO %jumpi(wnaf_end)
    // stack: o, segment, n, retdest
    %jump(wnaf_loop)

wnaf_end:
    // stack: o, segment, n, retdest
    %pop3 JUMP

wnaf_zero_scalar:
    // stack: segment, n, retdest
    %pop2 JUMP



// Number of trailing zeros computed with a simple loop and returning the scalar without its lsb zeros.
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
    // stack: count, x>>1, retdest
    %jump(trailing_zeros_loop)
trailing_zeros_end:
    %stack (count, x, retdest) -> (retdest, x, count)
    JUMP
