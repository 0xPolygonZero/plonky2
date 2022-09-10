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
///     return mix(state, stateL, stateR)
///
///
/// def mix(*stateR, *stateL, *state):
///     return
///         u32(state[1] + stateL[2] + stateR[3]),
///         u32(state[2] + stateL[3] + stateR[4]),
///         u32(state[3] + stateL[4] + stateR[0]),
///         u32(state[4] + stateL[0] + stateR[1]),
///         u32(state[0] + stateL[1] + stateR[2])
/// 
/// In mix, we denote state[i], stateL[i], stateR[i] by si, li, ri

global hash:
    JUMPDEST
    // stack: *state, retdest
    PUSH switch  
    PUSH 1
    PUSH 5  
    PUSH 16  
    PUSH 0  
    PUSH 0
    // stack: 0, 0, 16, 5, 1, switch, *state, retdest
    DUP11  
    DUP11  
    DUP11  
    DUP11  
    DUP11
    // stack: *state, 0, 0, 16, 5, 1, switch, *state, retdest
    %jump(loop)
switch:
    JUMPDEST
    // stack: *stateL, *state, retdest
    PUSH mix  
    PUSH 0  
    PUSH 5  
    PUSH 16  
    PUSH 0  
    PUSH 0
    // stack: 0, 0, 16, 5, 0, mix, *stateL, *state, retdest
    DUP16  
    DUP16  
    DUP16  
    DUP16  
    DUP16
    // stack: *state, 0, 0, 16, 5, 0, mix, *stateL, *state, retdest
    %jump(loop)
mix:
    JUMPDEST
    // stack: r0, r1, r2, r3, r4, l0, l1, l2, l3, l4, s0, s1, s2, s3, s4, retdest
    SWAP10
    // stack: s0, r1, r2, r3, r4, l0, l1, l2, l3, l4, r0, s1, s2, s3, s4, retdest
    SWAP1
    // stack: r1, s0, r2, r3, r4, l0, l1, l2, l3, l4, r0, s1, s2, s3, s4, retdest
    SWAP6
    // stack: l1, s0, r2, r3, r4, l0, r1, l2, l3, l4, r0, s1, s2, s3, s4, retdest
    %add3_32
    // stack: s0+l1+r2, r3, r4, l0, r1, l2, l3, l4, r0, s1, s2, s3, s4, retdest
    SWAP13
    // stack: retdest, r3, r4, l0, r1, l2, l3, l4, r0, s1, s2, s3, s4, s0+l1+r2
    SWAP11
    // stack: s3, r3, r4, l0, r1, l2, l3, l4, r0, s1, s2, retdest, s4, s0+l1+r2
    SWAP10
    // stack: s2, r3, r4, l0, r1, l2, l3, l4, r0, s1, s3, retdest, s4, s0+l1+r2
    SWAP1
    // stack: r3, s2, r4, l0, r1, l2, l3, l4, r0, s1, s3, retdest, s4, s0+l1+r2
    SWAP6
    // stack: l3, s2, r4, l0, r1, l2, r3, l4, r0, s1, s3, retdest, s4, s0+l1+r2
    %add3_32
    // stack: s2+l3+r4, l0, r1, l2, r3, l4, r0, s1, s3, retdest, s4, s0+l1+r2
    SWAP8
    // stack: s3, l0, r1, l2, r3, l4, r0, s1, s2+l3+r4, retdest, s4, s0+l1+r2
    SWAP10
    // stack: s4, l0, r1, l2, r3, l4, r0, s1, s2+l3+r4, retdest, s3, s0+l1+r2
    %add3_32
    // stack: s4+l0+r1, l2, r3, l4, r0, s1, s2+l3+r4, retdest, s3, s0+l1+r2
    SWAP8
    // stack: s3, l2, r3, l4, r0, s1, s2+l3+r4, retdest, s4+l0+r1, s0+l1+r2
    SWAP5
    // stack: s1, l2, r3, l4, r0, s3, s2+l3+r4, retdest, s4+l0+r1, s0+l1+r2
    %add3_32
    // stack: s1+l2+r3, l4, r0, s3, s2+l3+r4, retdest, s4+l0+r1, s0+l1+r2
    SWAP3
    // stack: s3, l4, r0, s1+l2+r3, s2+l3+r4, retdest, s4+l0+r1, s0+l1+r2
    %add3_32
    // stack: s3+l4+r0, s1+l2+r3, s2+l3+r4, retdest, s4+l0+r1, s0+l1+r2
    SWAP3
    // stack: retdest, s1+l2+r3, s2+l3+r4, s3+l4+r0, s4+l0+r1, s0+l1+r2
    JUMP


/// def loop(*state):
///     while rounds:
///         update_round_vars()
///         round(*state, F, K, rounds, sides)
///
/// def update_round_vars():
///     F = load_F(sides, rounds)
///     K = load_K(sides, rounds)
///
/// def round(*state, rounds, sides):
///     while boxes:
///         box(*state, F, K)
///         boxes -= 1
///     boxes   = 16
///     rounds -= 1


loop:
    JUMPDEST
    // stack:          *state, F, K, 16, rounds, sides, retdest
    DUP9
    // stack:   round, *state, F, K, 16, rounds, sides, retdest
    %jumpi(update_round_vars)
    // stack:          *state, F, K, 16,      0, sides, retdest
    %stack (a, b, c, d, e, F, K, boxes, rounds, sides, retdest) -> (retdest, a, b, c, d, e)
    // stack: retdest, *state
    JUMP
update_round_vars:
    JUMPDEST
    // stack:           *state, F , K , 16, rounds, sides, retdest
    DUP9  
    DUP11  
    %get_round  
    DUP1
    // stack: rnd, rnd, *state, F , K , 16, rounds, sides, retdest
    SWAP7  
    POP  
    %push_F  
    SWAP7
    // stack: rnd, rnd, *state, F', K , 16, rounds, sides, retdest
    SWAP8  
    POP  
    %load_K  
    SWAP7  
    POP
    // stack:           *state, F', K', 16, rounds, sides, retdest
    %jump(round)
round:
    JUMPDEST
    // stack:        *state, F, K, boxes, rounds  , sides, retdest
    DUP8
    // stack: boxes, *state, F, K, boxes, rounds  , sides, retdest
    %jumpi(box)
    // stack:        *state, F, K,     0, rounds  , sides, retdest
    SWAP7  
    POP  
    PUSH 16 
    SWAP7
    // stack:        *state, F, K,    16, rounds  , sides, retdest
    PUSH 1  
    DUP10  
    SUB  
    SWAP9  
    POP
    // stack:        *state, F, K,    16, rounds-1, sides, retdest
    %jump(loop)


/// Note that we unpack *state to a, b, c, d, e 
/// All additions are u32
///
/// def box(a, b, c, d, e, F, K):
///
///     box = get_box(sides, rounds, boxes)
///     a  += F(b, c, d)
///     r   = load_r(box)
///     x   = load_block(r)
///     a  += x + K
///     s   = load_s(box)
///     a   = rol(s, a)
///     a  += e
///     c   = rol(10, c)
///
///     return e, a, b, c, d, F, K


box:
    JUMPDEST
    // stack:                      a, b, c, d, e, F, K, boxes, rounds, sides
    PUSH pre_rol  
    DUP5
    DUP5
    DUP5  
    DUP10
    // stack: F, b, c, d, pre_rol, a, b, c, d, e, F, K, boxes, rounds, sides
    JUMP
pre_rol:
    JUMPDEST
    // stack:   F(b, c, d), a, b, c, d, e, F, K, boxes, rounds, sides
    ADD
    // stack:               a, b, c, d, e, F, K, boxes, rounds, sides
    %get_box_from_stack
    // stack:          box, a, b, c, d, e, F, K, boxes, rounds, sides
    DUP1  
    %load_r
    // stack:       r, box, a, b, c, d, e, F, K, boxes, rounds, sides    
    %mload_kernel(@SEGMENT_KERNEL_GENERAL)
    // stack:       x, box, a, b, c, d, e, F, K, boxes, rounds, sides
    SWAP1  
    SWAP2 
    // stack:       a, x, box, b, c, d, e, F, K, boxes, rounds, sides
    ADD  
    DUP8  
    ADD  
    %u32
    // stack:          a, box, b, c, d, e, F, K, boxes, rounds, sides
    PUSH mid_rol  
    SWAP2
    // stack: box, a, mid_rol, b, c, d, e, F, K, boxes, rounds, sides
    %load_s
    // stack:   s, a, mid_rol, b, c, d, e, F, K, boxes, rounds, sides
    %jump(rol)
mid_rol:
    JUMPDEST
    // stack:               a, b, c, d, e, F, K, boxes, rounds, sides
    DUP5
    // stack:            e, a, b, c, d, e, F, K, boxes, rounds, sides
    ADD 
    %u32    
    // stack:               a, b, c, d, e, F, K, boxes, rounds, sides
    SWAP1  
    SWAP2  
    PUSH post_rol  
    SWAP1  
    PUSH 10
    // stack: 10, c, post_rol, b, a, d, e, F, K, boxes, rounds, sides
    %jump(rol)
post_rol:
    JUMPDEST
    // stack: c, a, b, d, e, F, K, boxes  , rounds, sides
    SWAP3
    // stack: d, a, b, c, e, F, K, boxes  , rounds, sides
    SWAP4
    // stack: e, a, b, c, d, F, K, boxes  , rounds, sides
    SWAP7  
    PUSH 1  
    SWAP1  
    SUB  
    SWAP7
    // stack: e, a, b, c, d, F, K, boxes-1, rounds, sides
    %jump(round)


%macro get_round
    // stack: sides, rounds
    %mul_const(5)  PUSH 10  sub  sub
    // stack: 10 - 5*sides - rounds
%endmacro


%macro get_box_from_stack
    // stack:                                     *7_args, boxes, rounds, sides
    DUP10  
    %mul_const(80)  
    DUP10  
    %mul_const(16)  
    DUP10  
    // stack:       boxes , 16*rounds , 80*sides, *7_args, boxes, rounds, sides
    PUSH 176  
    SUB  
    SUB  
    SUB
    // stack: 176 - boxes - 16*rounds - 80*sides, *7_args, boxes, rounds, sides
%endmacro
