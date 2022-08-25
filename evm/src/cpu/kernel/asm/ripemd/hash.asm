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
    jumpdest
    // stack: *state, retdest
    push switch  push 0  push 5  push 16  push 0  push 0
    // stack: 0, 0, 16, 5, 1, switch, *state, retdest
    dup11  dup11  dup11  dup11  dup11
    // stack: *state, 0, 0, 16, 5, 1, switch, *state, retdest
    %jump(loop)
switch:
    jumpdest
    // stack: *stateL, *state, retdest
    push mix  push 1  push 5  push 16  push 0  push 0
    // stack: 0, 0, 16, 5, 0, mix, *stateL, *state, retdest
    dup16  dup16  dup16  dup16  dup16
    // stack: *state, 0, 0, 16, 5, 0, mix, *stateL, *state, retdest
    %jump(loop)
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
    jump


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
    jumpdest
    // stack:          *state, F, K, 16, rounds, sides, retdest
    dup9
    // stack:   round, *state, F, K, 16, rounds, sides, retdest
    %jumpi(update_round_vars)
    // stack:          *state, F, K, 16,      0, sides, retdest
    %stack (a, b, c, d, e, F, K, boxes, rounds, sides, retdest) -> (retdest, a, b, c, d, e)
    // stack: retdest, *state
    jump
update_round_vars:
    jumpdest
    // stack:           *state, F , K , 16, rounds, sides, retdest
    dup10  dup10  %get_round  dup1
    // stack: rnd, rnd, *state, F , K , 16, rounds, sides, retdest
    swap7  pop  %push_F  swap7
    // stack: rnd, rnd, *state, F', K , 16, rounds, sides, retdest
    swap8  pop  %load_K  swap7  pop
    // stack:           *state, F', K', 16, rounds, sides, retdest
    %jump(round)
round:
    jumpdest
    // stack:        *state, F, K, boxes, rounds  , sides, retdest
    dup8
    // stack: boxes, *state, F, K, boxes, rounds  , sides, retdest
    %jumpi(box)
    // stack:        *state, F, K,     0, rounds  , sides, retdest
    swap7  pop  push 16  swap7
    // stack:        *state, F, K,    16, rounds  , sides, retdest
    push 1  dup10  sub  swap9  pop
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
///     a   = ROL(s, a)
///     a  += e
///     c   = ROL(10, c)
///
///     return e, a, b, c, d, F, K


box:
    jumpdest
    // stack:                      a, b, c, d, e, F, K, boxes, rounds, sides
    push after_F  dup5  dup5  dup5  dup10
    // stack: F, b, c, d, pre_rol, a, b, c, d, e, F, K, boxes, rounds, sides
    jump
pre_rol:
    jumpdest
    // stack:   F(b, c, d), a, b, c, d, e, F, K, boxes, rounds, sides
    add
    // stack:               a, b, c, d, e, F, K, boxes, rounds, sides
    %get_box_from_stack
    // stack:          box, a, b, c, d, e, F, K, boxes, rounds, sides
    dup1  %load_r
    // stack:       r, box, a, b, c, d, e, F, K, boxes, rounds, sides    
    %mload_kernel(@SEGMENT_KERNEL_GENERAL)
    // stack:       x, box, a, b, c, d, e, F, K, boxes, rounds, sides
    swap1  swap2 
    // stack:       a, x, box, b, c, d, e, F, K, boxes, rounds, sides
    add  dup8  add  %u32
    // stack:          a, box, b, c, d, e, F, K, boxes, rounds, sides
    push mid_rol  swap2
    // stack: box, a, mid_rol, b, c, d, e, F, K, boxes, rounds, sides
    %load_s
    // stack:   s, a, mid_rol, b, c, d, e, F, K, boxes, rounds, sides
    %jump(ROL)
mid_rol:
    jumpdest
    // stack:               a, b, c, d, e, F, K, boxes, rounds, sides
    dup5
    // stack:            e, a, b, c, d, e, F, K, boxes, rounds, sides
    add %u32    
    // stack:               a, b, c, d, e, F, K, boxes, rounds, sides
    %stack (a, b, c) -> (10, c, post_rol, a, b)
    // stack: 10, c, post_rol, b, a, d, e, F, K, boxes, rounds, sides
    %jump(ROL)
post_rol:
    jumpdest
    // stack: c, a, b, d, e, F, K, boxes  , rounds, sides
    swap4
    // stack: d, a, b, c, e, F, K, boxes  , rounds, sides
    swap5
    // stack: e, a, b, c, d, F, K, boxes  , rounds, sides
    swap7  push 1  swap1  sub  swap7
    // stack: e, a, b, c, d, F, K, boxes-1, rounds, sides
    %jump(round)


%macro get_round
    // stack: sides, rounds
    %mul_const(5)  push 10  sub  sub
    // stack: 10 - 5*sides - rounds
%end_macro


%macro get_box_from_stack
    // stack:                                     *7_args, boxes, rounds, sides
    dup10  %mul_const(80)  dup10  %mul_const(16)  dup10  
    // stack:       boxes , 16*rounds , 80*sides, *7_args, boxes, rounds, sides
    push 160  sub  sub  sub
    // stack: 160 - boxes - 16*rounds - 80*sides, *7_args, boxes, rounds, sides
%end_macro
