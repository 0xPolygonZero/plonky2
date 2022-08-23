/// Variables beginning with _ are not maintained on the stack
/// Note that state takes up 5 stack slots


/// def hash(state, _block):
/// 
///     stateL = state
///     stateL = loop(stateL)
///
///     stateR = state
///     stateR = loop(stateR)
///
///     state = mix(state, stateL, stateR)

global hash:
    jumpdest
    // stack: *state, retdest
    push switch push 5 push 16 push K0 push F0
    // stack: F0, K0, 16, 5, switch, *state, retdest
    dup10 dup10 dup10 dup10 dup10
    // stack: *state, F0, K0, 16, 5, switch, *state, retdest
    %jump(loop)
switch:
    jumpdest
    // stack: *stateL, *state, retdest
    push mix push 5 push 16
    // stack: F0, K0, 16, 5, mix, *stateL, *state, retdest
    dup15 dup15 dup15 dup15 dup15
    // stack: *state, F0, K0, 16, 5, mix, *stateL, *state, retdest
    %jump(loop)


/// def loop(*state, Fj, Kj):
///     while n:
///         while i:
///             R(*state, Fj, Kj)
///             i -= 1
///         i = 16
///         n -= 1
///         F = Fs[n]
///         K = Ks[n]

loop:
    jumpdest
    // stack: *stack, Fj, Kj, 16, n, retdest
    push 1 dup9 sub swap8
    // stack: n, *stack, Fj, Kj, 16, n-1, retdest
    %jumpi(cycle)
    // stack: *stack, Fj, Kj, 16, -1, retdest
    %stack (a, b, c, d, e, f, k, i, n, ret) -> (ret, a, b, c, d, e)
    // stack: retdest, *stack
    jump
cycle:
    jumpdest
    // stack: *stack, Fj, Kj, i, n, retdest
    push 1 dup9 sub swap8
    // stack: i, *stack, Fj, Kj, i-1, n, retdest
    %jumpi(R)
    // stack: *stack, Fj, Kj, -1, n, retdest
    swap5 pop push Fj swap5 ---------------------------------------------------------------------TODO
    // stack: *stack, Fj, Kj 16, n, retdest
    swap6 pop push Kj swap6 ---------------------------------------------------------------------TODO
    // stack: *stack, Fj, Kj 16, n, retdest
    swap7 pop push 16 swap7
    // stack: *stack, Fj, Kj 16, n, retdest
    %jump(loop)


/// def R(a, b, c, d, e, Fj, Kj, _sj, _rj, _X):
///     a = u32(ROL(sj, u32(Fj(b, c, d) + a + X[rj] + Kj)) + e)
///     c = ROL(10, c)
///     return e, a, b, c, d, Fj, Kj

R:
    jumpdest
    // stack: a, b, c, d, e, Fj, Kj
    push after_F dup5 dup5 dup5 dup10
    // stack: Fj, b, c, d, after_F, a, b, c, d, e, Fj, Kj
    jump
after_F:
    // stack: Fj(b, c, d), a, b, c, d, e, Fj, Kj
    add
    // stack: Fj(b, c, d) + a, b, c, d, e, Fj, Kj
    push X[rj]---------------------------------------------------------------------TODO
    // stack: X[rj], Fj(b, c, d) + a, b, c, d, e, Fj, Kj
    add
    // stack: X[rj] + Fj(b, c, d) + a, b, c, d, e, Fj, Kj
    dup7
    // stack: Kj, X[rj] + Fj(b, c, d) + a, b, c, d, e, Fj, Kj
    add %u32
    // stack: Kj + X[rj] + Fj(b, c, d) + a, b, c, d, e, Fj, Kj
    push sj------------------------------------------------------------------------TODO
    // stack: sj, Kj + X[rj] + Fj(b, c, d) + a, b, c, d, e, Fj, Kj
    %jump(ROL)
    // stack: ROL(sj, Kj + X[rj] + Fj(b, c, d) + a), b, c, d, e, Fj, Kj
    dup5
    // stack: e, ROL(sj, Kj + X[rj] + Fj(b, c, d) + a), b, c, d, e, Fj, Kj
    add %u32    
    // stack: e + ROL(sj, Kj + X[rj] + Fj(b, c, d) + a), b, c, d, e, Fj, Kj
    swap1 
    // stack: b, e + ROL(sj, Kj + X[rj] + Fj(b, c, d) + a), c, d, e, Fj, Kj
    swap2
    // stack: c, e + ROL(sj, Kj + X[rj] + Fj(b, c, d) + a), b, d, e, Fj, Kj
    push 10
    // stack: 10, c, b, e + ROL(sj, Kj + X[rj] + Fj(b, c, d) + a), d, e, Fj, Kj
    %jump(ROL)
    // stack: ROL(10, c), e + ROL(sj, Kj + X[rj] + Fj(b, c, d) + a), b, d, e, Fj, Kj
    swap4
    // stack: d, e + ROL(sj, Kj + X[rj] + Fj(b, c, d) + a), b, ROL(10,c), e, Fj, Kj
    swap5
    // stack: e, e + ROL(sj, Kj + X[rj] + Fj(b, c, d) + a), b, ROL(10, c), d, Fj, Kj
    %jump(cycle)


/// def mix(*stateR, *stateL, *state):
///     return [
///     u32(state[1] + stateL[2] + stateR[3]),
///     u32(state[2] + stateL[3] + stateR[4]),
///     u32(state[3] + stateL[4] + stateR[0]),
///     u32(state[4] + stateL[0] + stateR[1]),
///     u32(state[0] + stateL[1] + stateR[2])
///     ]
/// 
/// Note that we denote state[i], stateL[i], stateR[i] by si, li, ri

mix:
    jumpdest
    // stack: r0, r1, r2, r3, r4, l0, l1, l2, l3, l4, s0, s1, s2, s3, s4, retdest
    swap10
    // stack: s0, r1, r2, r3, r4, l0, l1, l2, l3, l4, r0, s1, s2, s3, s4, retdest
    swap1
    // stack: r1, s0, r2, r3, r4, l0, l1, l2, l3, l4, r0, s1, s2, s3, s4, retdest
    swap6
    // stack: l1, s0, r2, r3, r4, l0, r1, l2, l3, l4, r0, s1, s2, s3, s4, retdest
    %add3_32
    // stack: s0+l1+r2, r3, r4, l0, r1, l2, l3, l4, r0, s1, s2, s3, s4, retdest
    swap13
    // stack: retdest, r3, r4, l0, r1, l2, l3, l4, r0, s1, s2, s3, s4, s0+l1+r2
    swap11
    // stack: s3, r3, r4, l0, r1, l2, l3, l4, r0, s1, s2, retdest, s4, s0+l1+r2
    swap10
    // stack: s2, r3, r4, l0, r1, l2, l3, l4, r0, s1, s3, retdest, s4, s0+l1+r2
    swap1
    // stack: r3, s2, r4, l0, r1, l2, l3, l4, r0, s1, s3, retdest, s4, s0+l1+r2
    swap6
    // stack: l3, s2, r4, l0, r1, l2, r3, l4, r0, s1, s3, retdest, s4, s0+l1+r2
    %add3_32
    // stack: s2+l3+r4, l0, r1, l2, r3, l4, r0, s1, s3, retdest, s4, s0+l1+r2
    swap8
    // stack: s3, l0, r1, l2, r3, l4, r0, s1, s2+l3+r4, retdest, s4, s0+l1+r2
    swap10
    // stack: s4, l0, r1, l2, r3, l4, r0, s1, s2+l3+r4, retdest, s3, s0+l1+r2
    %add3_32
    // stack: s4+l0+r1, l2, r3, l4, r0, s1, s2+l3+r4, retdest, s3, s0+l1+r2
    swap8
    // stack: s3, l2, r3, l4, r0, s1, s2+l3+r4, retdest, s4+l0+r1, s0+l1+r2
    swap5
    // stack: s1, l2, r3, l4, r0, s3, s2+l3+r4, retdest, s4+l0+r1, s0+l1+r2
    %add3_32
    // stack: s1+l2+r3, l4, r0, s3, s2+l3+r4, retdest, s4+l0+r1, s0+l1+r2
    swap3
    // stack: s3, l4, r0, s1+l2+r3, s2+l3+r4, retdest, s4+l0+r1, s0+l1+r2
    %add3_32
    // stack: s3+l4+r0, s1+l2+r3, s2+l3+r4, retdest, s4+l0+r1, s0+l1+r2
    swap3
    // stack: retdest, s1+l2+r3, s2+l3+r4, s3+l4+r0, s4+l0+r1, s0+l1+r2
