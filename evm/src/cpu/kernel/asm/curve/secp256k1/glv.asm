// Inspired by https://github.com/AztecProtocol/weierstrudel/blob/master/huff_modules/endomorphism.huff
// See also Sage code in evm/src/cpu/kernel/tests/ecc/glv_test_data
// Given scalar `k ∈ Secp256k1::ScalarField`, return `u, k1, k2` with `k1,k2 < 2^129` and such that
// `k = k1 - s*k2` if `u==0` otherwise `k = k1 + s*k2`, where `s` is the scalar value representing the endomorphism.
// In the comments below, N means @SECP_SCALAR
// TODO: write proof that outputs are <=129-bit
global glv_decompose:
    // stack: k, retdest
    PUSH @SECP_SCALAR DUP1 DUP1
    // Compute c2 which is the top 256 bits of k*g1. Use asm from https://medium.com/wicketh/mathemagic-full-multiply-27650fec525d.
    PUSH 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
    // stack: -1, N, N, N, k, retdest
    PUSH 0xe4437ed6010e88286f547fa90abfe4c4 DUP6
    // stack: k, g1, -1, N, N, N, k, retdest
    MULMOD
    // stack: (k * g1 % -1), N, N, N, k, retdest
    PUSH 0xe4437ed6010e88286f547fa90abfe4c4 DUP6
    // stack: k, g1, (k * g1 % -1), N, N, N, k, retdest
    MUL
    // stack: bottom = (k * g1), (k * g1 % -1), N, N, N, k, retdest
    DUP1 DUP3
    // stack: (k * g1 % -1), bottom, bottom, (k * g1 % -1), N, N, N, k, retdest
    LT SWAP2 SUB SUB
    // stack: c2, N, N, N, k, retdest
    PUSH 0x3086d221a7d46bcde86c90e49284eb15 MULMOD
    // stack: q2=c2*b2, N, N, k, retdest

    // Use the same trick to compute c1 = top 256 bits of g2*k.
    PUSH @SECP_SCALAR PUSH 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
    PUSH 0x3086d221a7d46bcde86c90e49284eb15 DUP7 MULMOD
    PUSH 0x3086d221a7d46bcde86c90e49284eb15 DUP7 MUL
    DUP1 DUP3 LT
    SWAP2 SUB SUB
    // stack: c1, N, q2, N, N, k, retdest
    PUSH 0xfffffffffffffffffffffffffffffffdd66b5e10ae3a1813507ddee3c5765c7e MULMOD
    // stack: q1, q2, N, N, k, retdest

    // We compute k2 = q1 + q2 - N, but we check for underflow and return N-q1-q2 instead if there is one,
    // along with a flag `underflow` set to 1 if there is an underflow, 0 otherwise.
    ADD %sub_check_underflow
    // stack: k2, underflow, N, k, retdest
    SWAP3 PUSH @SECP_SCALAR DUP5 PUSH 0x5363ad4cc05c30e0a5261c028812645a122e22ea20816678df02967c1b23bd72
    // stack: s, k2, N, k, underflow, N, k2, retdest
    MULMOD
    // stack: s *k2, k, underflow, N, k2, retdest
    // Need to return `k + s*k2` if no underflowed occur, otherwise return `k - s*k2` which is done in the `underflowed` fn.
    SWAP2 DUP1 %jumpi(underflowed)
    %stack (underflow, k, x, N, k2) -> (k, x, N, k2, underflow)
    ADDMOD
    %stack (k1, k2, underflow, retdest) -> (retdest, underflow, k1, k2)
    JUMP

underflowed:
    // stack: underflow, k, s*k2, N, k2
    // Compute (k-s*k2)%N. TODO: Use SUBMOD here when ready
    %stack (u, k, x, N, k2) -> (N, x, k, N, k2, u)
    SUB ADDMOD
    %stack (k1, k2, underflow, retdest) -> (retdest, underflow, k1, k2)
    JUMP

%macro sub_check_underflow
    // stack: x, y
    DUP2 DUP2 LT
    // stack: x<y, x, y
    DUP1 ISZERO DUP2 DUP4 DUP6 SUB MUL
    // stack: (y-x)*(x<y), x>=y, x<y, x, y
    %stack (a, b, c, x, y) -> (x, y, b, a, c)
    SUB MUL ADD
    // stack: x-y if x>=y else y-x, x<y
%endmacro