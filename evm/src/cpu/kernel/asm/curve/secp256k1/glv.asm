// Inspired by https://github.com/AztecProtocol/weierstrudel/blob/master/huff_modules/endomorphism.huff
// See also Sage code in evm/src/cpu/kernel/tests/ecc/secp_glv_test_data
// Given scalar `k ∈ Secp256k1::ScalarField`, return `u, k1, k2` with `k1,k2 < 2^129` and such that
// `k = k1 - s*k2` if `u==0` otherwise `k = k1 + s*k2`, where `s` is the scalar value representing the endomorphism.
// In the comments below, N means @SECP_SCALAR
//
// Z3 proof that the resulting `k1, k2` satisfy `k1>0`, `k1 < 2^129` and `|k2| < 2^129`.
// ```python
// from z3 import Solver, Int, Or, unsat
// q = 115792089237316195423570985008687907852837564279074904382605163141518161494337
// glv_s = 37718080363155996902926221483475020450927657555482586988616620542887997980018
// g1 = 303414439467246543595250775667605759172
// g2 = 64502973549206556628585045361533709077
// b2 = 64502973549206556628585045361533709077
// b1 = -303414439467246543595250775667605759171
// k = Int("k")
// c1 = Int("c1")
// c2 = Int("c2")
// s = Solver()
//
// c2p = -c2
// s.add(k < q)
// s.add(0 < k)
// s.add(c1 * (2**256) <= g2 * k)
// s.add((c1 + 1) * (2**256) > g2 * k)
// s.add(c2p * (2**256) <= g1 * k)
// s.add((c2p + 1) * (2**256) > g1 * k)
//
// q1 = c1 * b1
// q2 = c2 * b2
//
// k2 = q2 - q1
// k2L = (glv_s * k2) % q
// k1 = k - k2L
//
// s.add(Or((k2 >= 2**129), (-k2 >= 2**129), (k1 >= 2**129), (k1 < 0)))
// assert s.check() == unsat
// ```
global secp_glv_decompose:
    // stack: k, retdest
    PUSH @SECP_SCALAR DUP1 DUP1
    // Compute c2 which is the top 256 bits of k*g1. Use asm from https://medium.com/wicketh/mathemagic-full-multiply-27650fec525d.
    PUSH @U256_MAX
    // stack: -1, N, N, N, k, retdest
    PUSH @SECP_GLV_MINUS_G1 DUP6
    // stack: k, g1, -1, N, N, N, k, retdest
    MULMOD
    // stack: (k * g1 % -1), N, N, N, k, retdest
    PUSH @SECP_GLV_MINUS_G1 DUP6
    // stack: k, g1, (k * g1 % -1), N, N, N, k, retdest
    MUL
    // stack: bottom = (k * g1), (k * g1 % -1), N, N, N, k, retdest
    DUP1 DUP3
    // stack: (k * g1 % -1), bottom, bottom, (k * g1 % -1), N, N, N, k, retdest
    LT SWAP2 SUB SUB
    // stack: c2, N, N, N, k, retdest
    PUSH @SECP_GLV_B2 MULMOD
    // stack: q2=c2*b2, N, N, k, retdest

    // Use the same trick to compute c1 = top 256 bits of g2*k.
    PUSH @SECP_SCALAR PUSH @U256_MAX
    PUSH @SECP_GLV_G2 DUP7 MULMOD
    PUSH @SECP_GLV_G2 DUP7 MUL
    DUP1 DUP3 LT
    SWAP2 SUB SUB
    // stack: c1, N, q2, N, N, k, retdest
    PUSH @SECP_GLV_B1 MULMOD
    // stack: q1, q2, N, N, k, retdest

    // We compute k2 = q1 + q2 - N, but we check for underflow and return N-q1-q2 instead if there is one,
    // along with a flag `underflow` set to 1 if there is an underflow, 0 otherwise.
    ADD %sub_check_underflow
    // stack: k2, underflow, N, k, retdest
    SWAP3 PUSH @SECP_SCALAR DUP5 PUSH @SECP_GLV_S
    // stack: s, k2, N, k, underflow, N, k2, retdest
    MULMOD
    // stack: s*k2, k, underflow, N, k2, retdest
    // Need to return `k + s*k2` if no underflow occur, otherwise return `k - s*k2` which is done in the `underflowed` fn.
    SWAP2 DUP1 %jumpi(underflowed)
    %stack (underflow, k, x, N, k2) -> (k, x, N, k2, underflow)
    ADDMOD
    %stack (k1, k2, underflow, retdest) -> (retdest, underflow, k1, k2)
    JUMP

underflowed:
    // stack: underflow, k, s*k2, N, k2
    // Compute (k-s*k2)%N.
    %stack (u, k, x, N, k2) -> (k, x, N, k2, u)
    SUBMOD
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
    %stack (res, bool) -> (res, @SECP_SCALAR, bool)
    MOD
%endmacro

