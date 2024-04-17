// Inspired by https://github.com/AztecProtocol/weierstrudel/blob/master/huff_modules/endomorphism.huff
// See also Sage code in evm/src/cpu/kernel/tests/ecc/bn_glv_test_data
// Given scalar `k ∈ Bn254::ScalarField`, return `u, k1, k2` with `k1,k2 < 2^127` and such that
// `k = k1 - s*k2` if `u==0` otherwise `k = k1 + s*k2`, where `s` is the scalar value representing the endomorphism.
// In the comments below, N means @BN_SCALAR
//
// Z3 proof that the resulting `k1, k2` satisfy `k1>0`, `k1 < 2^127` and `|k2| < 2^127`.
// ```python
// from z3 import Solver, Int, Or, unsat
// q = 0x30644E72E131A029B85045B68181585D2833E84879B9709143E1F593F0000001
// glv_s = 0xB3C4D79D41A917585BFC41088D8DAAA78B17EA66B99C90DD
//
// b2 = 0x89D3256894D213E3
// b1 = -0x6F4D8248EEB859FC8211BBEB7D4F1128
//
// g1 = 0x24CCEF014A773D2CF7A7BD9D4391EB18D
// g2 = 0x2D91D232EC7E0B3D7
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
// k2 = -k2
//
// s.add(Or((k2 >= 2**127), (-k2 >= 2**127), (k1 >= 2**127), (k1 < 0)))
//
// assert s.check() == unsat
// ```
global bn_glv_decompose:
    // stack: k, retdest
    %mod_const(@BN_SCALAR)
    PUSH @BN_SCALAR DUP1 DUP1
    // Compute c2 which is the top 256 bits of k*g1. Use asm from https://medium.com/wicketh/mathemagic-full-multiply-27650fec525d.
    PUSH @U256_MAX
    // stack: -1, N, N, N, k, retdest
    PUSH @BN_GLV_MINUS_G1 DUP6
    // stack: k, g1, -1, N, N, N, k, retdest
    MULMOD
    // stack: (k * g1 % -1), N, N, N, k, retdest
    PUSH @BN_GLV_MINUS_G1 DUP6
    // stack: k, g1, (k * g1 % -1), N, N, N, k, retdest
    MUL
    // stack: bottom = (k * g1), (k * g1 % -1), N, N, N, k, retdest
    DUP1 DUP3
    // stack: (k * g1 % -1), bottom, bottom, (k * g1 % -1), N, N, N, k, retdest
    LT SWAP2 SUB SUB
    // stack: c2, N, N, N, k, retdest
    PUSH @BN_GLV_B2 MULMOD
    // stack: q2=c2*b2, N, N, k, retdest

    // Use the same trick to compute c1 = top 256 bits of g2*k.
    PUSH @BN_SCALAR PUSH @U256_MAX
    PUSH @BN_GLV_G2 DUP7 MULMOD
    PUSH @BN_GLV_G2 DUP7 MUL
    DUP1 DUP3 LT
    SWAP2 SUB SUB
    // stack: c1, N, q2, N, N, k, retdest
    PUSH @BN_GLV_B1 MULMOD
    // stack: q1, q2, N, N, k, retdest

    // We compute k2 = q1 + q2 - N, but we check for underflow and return N-q1-q2 instead if there is one,
    // along with a flag `underflow` set to 1 if there is an underflow, 0 otherwise.
    ADD %bn_sub_check_underflow
    // stack: k2, underflow, N, k, retdest
    DUP1 %ge_const(0x80000000000000000000000000000000) %jumpi(negate)
    %jump(contd)
negate:
    // stack: k2, underflow, N, k, retdest
    SWAP1 PUSH 1 SUB SWAP1
    PUSH @BN_SCALAR SUB
contd:
    // stack: k2, underflow, N, k, retdest
    SWAP3 PUSH @BN_SCALAR DUP5 PUSH @BN_GLV_S
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

%macro bn_sub_check_underflow
    // stack: x, y
    DUP2 DUP2 LT
    // stack: x<y, x, y
    DUP1 ISZERO DUP2 DUP4 DUP6 SUB MUL
    // stack: (y-x)*(x<y), x>=y, x<y, x, y
    %stack (a, b, c, x, y) -> (x, y, b, a, c)
    SUB MUL ADD
    %stack (res, bool) -> (res, @BN_SCALAR, bool)
    MOD
%endmacro
