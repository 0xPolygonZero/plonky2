/// Variables beginning with _ are not maintained on the stack
/// Note that state takes up 5 stack slots


/// def compression(state, _block):
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
///         u32(s1 + l2 + r3),
///         u32(s2 + l3 + r4),
///         u32(s3 + l4 + r0),
///         u32(s4 + l0 + r1),
///         u32(s0 + l1 + r2)
/// 
/// where si, li, ri, oi, BL, RD respectively denote 
/// state[i], stateL[i], stateR[i], output[i], block, retdest

global compression:
    // stack:                                         *state, block, retdest 
    PUSH switch
    DUP7
    PUSH 1
    PUSH 5  
    PUSH 16  
    PUSH 0  
    PUSH 0
    // stack:         0, 0, 16, 5, 1, block, switch, *state, block, retdest 
    DUP12  
    DUP12  
    DUP12  
    DUP12  
    DUP12
    // stack: *state, 0, 0, 16, 5, 1, block, switch, *state, block, retdest 
    %jump(loop)
switch:
    // stack:                                     *stateL, *state, block, retdest
    PUSH mix
    DUP12  
    PUSH 0
    PUSH 5  
    PUSH 16  
    PUSH 0  
    PUSH 0
    // stack:         0, 0, 16, 5, 0, block, mix, *stateL, *state, block, retdest
    DUP17  
    DUP17  
    DUP17  
    DUP17  
    DUP17
    // stack: *state, 0, 0, 16, 5, 0, block, mix, *stateL, *state, block, retdest 
    %jump(loop)
mix:
    // stack: r0, r1, r2, r3, r4, l0, l1, l2, l3, l4, s0, s1, s2, s3, s4, BL, RD 
    SWAP10
    // stack: s0, r1, r2, r3, r4, l0, l1, l2, l3, l4, r0, s1, s2, s3, s4, BL, RD 
    SWAP1
    // stack: r1, s0, r2, r3, r4, l0, l1, l2, l3, l4, r0, s1, s2, s3, s4, BL, RD 
    SWAP6
    // stack: l1, s0, r2, r3, r4, l0, r1, l2, l3, l4, r0, s1, s2, s3, s4, BL, RD 
    %add3_32
    // stack:         o4, r3, r4, l0, r1, l2, l3, l4, r0, s1, s2, s3, s4, BL, RD 
    SWAP14
    // stack:         RD, r3, r4, l0, r1, l2, l3, l4, r0, s1, s2, s3, s4, BL, o4 
    SWAP11
    // stack:         s3, r3, r4, l0, r1, l2, l3, l4, r0, s1, s2, RD, s4, BL, o4 
    SWAP10
    // stack:         s2, r3, r4, l0, r1, l2, l3, l4, r0, s1, s3, RD, s4, BL, o4 
    SWAP1
    // stack:         r3, s2, r4, l0, r1, l2, l3, l4, r0, s1, s3, RD, s4, BL, o4 
    SWAP6
    // stack:         l3, s2, r4, l0, r1, l2, r3, l4, r0, s1, s3, RD, s4, BL, o4 
    %add3_32
    // stack:                 o1, l0, r1, l2, r3, l4, r0, s1, s3, RD, s4, BL, o4 
    SWAP9
    // stack:                 RD, l0, r1, l2, r3, l4, r0, s1, s3, o1, s4, BL, o4 
    SWAP10
    // stack:                 s4, l0, r1, l2, r3, l4, r0, s1, s3, o1, RD, BL, o4 
    %add3_32
    // stack:                         o3, l2, r3, l4, r0, s1, s3, o1, RD, BL, o4 
    SWAP9
    // stack:                         BL, l2, r3, l4, r0, s1, s3, o1, RD, o3, o4 
    SWAP5
    // stack:                         s1, l2, r3, l4, r0, BL, s3, o1, RD, o3, o4 
    %add3_32
    // stack:                                 o0, l4, r0, BL, s3, o1, RD, o3, o4 
    SWAP4
    // stack:                                 s3, l4, r0, BL, o0, o1, RD, o3, o4 
    %add3_32 
    // stack:                                         o2, BL, o0, o1, RD, o3, o4 
    SWAP4
    // stack:                                         RD, BL, o0, o1, o2, o3, o4 
    SWAP1
    // stack:                                         BL, RD, o0, o1, o2, o3, o4 
    POP
    // stack:                                             RD, o0, o1, o2, o3, o4 
    JUMP


/// def loop(*state):
///     while rounds:
///         update_round_vars()
///         round(*state, F, K, rounds, sides)
///
/// def update_round_vars():
///     F = load(F)(sides, rounds)
///     K = load(K)(sides, rounds)
///
/// def round(*state, rounds, sides):
///     while boxes:
///         box(*state, F, K)
///         boxes -= 1
///     boxes   = 16
///     rounds -= 1


loop:  
    // stack:          *state, F, K, 16, rounds, sides, block, retdest
    DUP9
    // stack:   round, *state, F, K, 16, rounds, sides, block, retdest
    %jumpi(update_round_vars)
    // stack:          *state, F, K, 16,      0, sides, block, retdest
    %stack (a, b, c, d, e, F, K, boxes, rounds, sides, block, retdest) -> (retdest, a, b, c, d, e)
    // stack: retdest, *state
    JUMP
update_round_vars:
    // stack:           *state, F , K , 16, rounds, sides, block, retdest
    DUP9  
    DUP11  
    %get_round  
    DUP1
    // stack: rnd, rnd, *state, F , K , 16, rounds, sides, block, retdest
    SWAP7  
    POP  
    %push_F  
    SWAP7
    // stack: rnd, rnd, *state, F', K , 16, rounds, sides, block, retdest
    SWAP8  
    POP  
    %load_u32(K_data)
    SWAP7  
    POP
    // stack:           *state, F', K', 16, rounds, sides, block, retdest
    %jump(round)
round:
    // stack:        *state, F, K, boxes, rounds  , sides, block, retdest
    DUP8
    // stack: boxes, *state, F, K, boxes, rounds  , sides, block, retdest
    %jumpi(box)
    // stack:        *state, F, K,     0, rounds  , sides, block, retdest
    SWAP7  
    POP  
    PUSH 16 
    SWAP7
    // stack:        *state, F, K,    16, rounds  , sides, block, retdest
    PUSH 1  
    DUP10  
    SUB  
    SWAP9  
    POP
    // stack:        *state, F, K,    16, rounds-1, sides, block, retdest
    %jump(loop)


/// Note that we unpack *state to a, b, c, d, e 
/// All additions are u32
///
/// def box(a, b, c, d, e, F, K):
///
///     box = get_box(sides, rounds, boxes)
///     a  += F(b, c, d)
///     r   = load_byte(r)(box)
///     x   = load_block(r)
///     a  += x + K
///     s   = load_byte(s)(box)
///     a   = rol(s, a)
///     a  += e
///     c   = rol(10, c)
///
///     return e, a, b, c, d, F, K


box:
    // stack:                      a, b, c, d, e, F, K, boxes, rounds, sides, block
    PUSH pre_rol  
    DUP5
    DUP5
    DUP5  
    DUP10
    // stack: F, b, c, d, pre_rol, a, b, c, d, e, F, K, boxes, rounds, sides, block
    JUMP
pre_rol:
    // stack:    F(b, c, d), a, b, c, d, e, F, K, boxes, rounds, sides, block
    ADD
    // stack:                a, b, c, d, e, F, K, boxes, rounds, sides, block
    %get_box
    // stack:           box, a, b, c, d, e, F, K, boxes, rounds, sides, block
    DUP1
    %load_byte(R_data)
    DUP13
    // stack: block, r, box, a, b, c, d, e, F, K, boxes, rounds, sides, block    
    %load_block
    // stack:        x, box, a, b, c, d, e, F, K, boxes, rounds, sides, block
    SWAP1  
    SWAP2 
    // stack:        a, x, box, b, c, d, e, F, K, boxes, rounds, sides, block
    ADD  
    DUP8  
    ADD  
    %u32
    // stack:           a, box, b, c, d, e, F, K, boxes, rounds, sides, block
    PUSH mid_rol  
    SWAP2
    // stack:  box, a, mid_rol, b, c, d, e, F, K, boxes, rounds, sides, block
    %load_byte(S_data)
    // stack:    s, a, mid_rol, b, c, d, e, F, K, boxes, rounds, sides, block
    %jump(rol)
mid_rol:
    // stack:               a, b, c, d, e, F, K, boxes, rounds, sides, block
    DUP5
    // stack:            e, a, b, c, d, e, F, K, boxes, rounds, sides, block
    ADD 
    %u32    
    // stack:               a, b, c, d, e, F, K, boxes, rounds, sides, block
    SWAP1  
    SWAP2  
    PUSH post_rol  
    SWAP1  
    PUSH 10
    // stack: 10, c, post_rol, b, a, d, e, F, K, boxes, rounds, sides, block
    %jump(rol)
post_rol:
    // stack: c, a, b, d, e, F, K, boxes  , rounds, sides, block
    SWAP3
    // stack: d, a, b, c, e, F, K, boxes  , rounds, sides, block
    SWAP4
    // stack: e, a, b, c, d, F, K, boxes  , rounds, sides, block
    SWAP7  
    PUSH 1  
    SWAP1  
    SUB  
    SWAP7
    // stack: e, a, b, c, d, F, K, boxes-1, rounds, sides, block
    %jump(round)


%macro get_round
    // stack: sides, rounds
    %mul_const(5)  PUSH 10  sub  sub
    // stack: 10 - 5*sides - rounds
%endmacro


%macro get_box
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
