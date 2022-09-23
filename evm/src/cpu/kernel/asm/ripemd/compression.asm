/// _block is stored in memory and its address virt remains on the stack
/// Note that STATE takes up 5 stack slots
/// def compress(state, _block):
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
/// def mix(stateR, stateL, state):
///     return
///         u32(s1 + l2 + r3),
///         u32(s2 + l3 + r4),
///         u32(s3 + l4 + r0),
///         u32(s4 + l0 + r1),
///         u32(s0 + l1 + r2)
/// 
/// where si, li, ri, oi, OF, RD respectively denote 
/// state[i], stateL[i], stateR[i], output[i], virt, retdest

global compress:
    // stack:                                       STATE, virt, retdest
    PUSH switch
    DUP7
    PUSH 1
    PUSH 5  
    PUSH 16  
    PUSH 0  
    PUSH 0
    // stack:         0, 0, 16, 5, 1, virt, switch, STATE, virt, retdest
    DUP12  
    DUP12  
    DUP12  
    DUP12  
    DUP12
    // stack:  STATE, 0, 0, 16, 5, 1, virt, switch, STATE, virt, retdest 
    %jump(loop)
switch:
    // stack:                                   STATEL, STATE, virt, retdest
    PUSH mix
    DUP12 
    PUSH 0
    PUSH 5  
    PUSH 16 
    // stack:              16, 5, 0, virt, mix, STATEL, STATE, virt, retdest
    DUP15
    DUP15
    DUP15
    DUP15
    DUP15
    // stack: STATE,       16, 5, 0, virt, mix, STATEL, STATE, virt, retdest
    %stack (STATE: 5) -> (STATE, 0, 0)
    // stack: STATE, 0, 0, 16, 5, 0, virt, mix, STATEL, STATE, virt, retdest 
    %jump(loop)
mix:
    // stack: r0, r1, r2, r3, r4, l0, l1, l2, l3, l4, s0, s1, s2, s3, s4, VR, RD 
    SWAP10
    // stack: s0, r1, r2, r3, r4, l0, l1, l2, l3, l4, r0, s1, s2, s3, s4, VR, RD 
    SWAP1
    // stack: r1, s0, r2, r3, r4, l0, l1, l2, l3, l4, r0, s1, s2, s3, s4, VR, RD 
    SWAP6
    // stack: l1, s0, r2, r3, r4, l0, r1, l2, l3, l4, r0, s1, s2, s3, s4, VR, RD 
    %add3_u32
    // stack:         o4, r3, r4, l0, r1, l2, l3, l4, r0, s1, s2, s3, s4, VR, RD 
    SWAP14
    // stack:         RD, r3, r4, l0, r1, l2, l3, l4, r0, s1, s2, s3, s4, VR, o4 
    SWAP11
    // stack:         s3, r3, r4, l0, r1, l2, l3, l4, r0, s1, s2, RD, s4, VR, o4 
    SWAP10
    // stack:         s2, r3, r4, l0, r1, l2, l3, l4, r0, s1, s3, RD, s4, VR, o4 
    SWAP1
    // stack:         r3, s2, r4, l0, r1, l2, l3, l4, r0, s1, s3, RD, s4, VR, o4 
    SWAP6
    // stack:         l3, s2, r4, l0, r1, l2, r3, l4, r0, s1, s3, RD, s4, VR, o4 
    %add3_u32
    // stack:                 o1, l0, r1, l2, r3, l4, r0, s1, s3, RD, s4, VR, o4 
    SWAP9
    // stack:                 RD, l0, r1, l2, r3, l4, r0, s1, s3, o1, s4, VR, o4 
    SWAP10
    // stack:                 s4, l0, r1, l2, r3, l4, r0, s1, s3, o1, RD, VR, o4 
    %add3_u32
    // stack:                         o3, l2, r3, l4, r0, s1, s3, o1, RD, VR, o4 
    SWAP9
    // stack:                         VR, l2, r3, l4, r0, s1, s3, o1, RD, o3, o4 
    SWAP5
    // stack:                         s1, l2, r3, l4, r0, VR, s3, o1, RD, o3, o4 
    %add3_u32
    // stack:                                 o0, l4, r0, VR, s3, o1, RD, o3, o4 
    SWAP4
    // stack:                                 s3, l4, r0, VR, o0, o1, RD, o3, o4 
    %add3_u32 
    // stack:                                         o2, VR, o0, o1, RD, o3, o4 
    SWAP4
    // stack:                                         RD, VR, o0, o1, o2, o3, o4 
    SWAP1
    // stack:                                         VR, RD, o0, o1, o2, o3, o4 
    POP
    // stack:                                             RD, o0, o1, o2, o3, o4
    JUMP


/// def loop(STATE):
///     while rounds:
///         update_round_vars()
///         round(STATE, F, K, rounds, sides)
///
/// def update_round_vars():
///     F = load(F)(sides, rounds)
///     K = load(K)(sides, rounds)
///
/// def round(STATE, rounds, sides):
///     while boxes:
///         box(STATE, F, K)
///         boxes -= 1
///     boxes   = 16
///     rounds -= 1


loop:  
    // stack:          STATE, F, K, 16, rounds, sides, virt, retdest
    DUP9
    // stack:   round, STATE, F, K, 16, rounds, sides, virt, retdest
    %jumpi(update_round_vars)
    // stack:          STATE, F, K, 16,      0, sides, virt, retdest
    %stack (STATE: 5, F, K, boxes, rounds, sides, virt, retdest) -> (retdest, STATE)
    // stack: retdest, STATE
    JUMP
update_round_vars:
    // stack:           STATE, F , K , 16, rounds, sides, virt, retdest
    DUP9  
    DUP11  
    %get_round  
    DUP1
    // stack: rnd, rnd, STATE, F , K , 16, rounds, sides, virt, retdest
    SWAP7  
    POP  
    %push_f  
    SWAP7
    // stack: rnd, rnd, STATE, F', K , 16, rounds, sides, virt, retdest
    SWAP8  
    POP  
    %mul_const(4)
    %mload_kernel_code_label_u32(K_data)
    SWAP7  
    POP
    // stack:           STATE, F', K', 16, rounds, sides, virt, retdest
    %jump(round)
round:
    // stack:        STATE, F, K, boxes, rounds  , sides, virt, retdest
    DUP8
    // stack: boxes, STATE, F, K, boxes, rounds  , sides, virt, retdest
    %jumpi(box)
    // stack:        STATE, F, K,     0, rounds  , sides, virt, retdest
    SWAP7  
    POP  
    PUSH 16 
    SWAP7
    // stack:        STATE, F, K,    16, rounds  , sides, virt, retdest
    PUSH 1  
    DUP10  
    SUB  
    SWAP9  
    POP
    // stack:        STATE, F, K,    16, rounds-1, sides, virt, retdest
    %jump(loop)


/// Note that we unpack STATE to a, b, c, d, e 
/// All additions are u32
///
/// def box(a, b, c, d, e, F, K):
///
///     box = get_box(sides, rounds, boxes)
///     a  += F(b, c, d)
///     r   = load(r)(box)
///     x   = load_offset(r)
///     a  += x + K
///     s   = load(s)(box)
///     a   = rol(s, a)
///     a  += e
///     c   = rol(10, c)
///
///     return e, a, b, c, d, F, K


box:
    // stack:                      a, b, c, d, e, F, K, boxes, rounds, sides, virt
    PUSH pre_rol  
    DUP5
    DUP5
    DUP5  
    DUP10
    // stack: F, b, c, d, pre_rol, a, b, c, d, e, F, K, boxes, rounds, sides, virt
    JUMP
pre_rol:
    // stack:    F(b, c, d), a, b, c, d, e, F, K, boxes, rounds, sides, virt
    ADD
    // stack:                a, b, c, d, e, F, K, boxes, rounds, sides, virt
    %get_box
    // stack:           box, a, b, c, d, e, F, K, boxes, rounds, sides, virt
    DUP12
    DUP2
    %mload_kernel_code_label(R_data)
    ADD
    // stack: virt + r, box, a, b, c, d, e, F, K, boxes, rounds, sides, virt  
    // %mload_kernel_code_u32_LE(Input_Block) 
    %load_u32_from_block
    // stack:        x, box, a, b, c, d, e, F, K, boxes, rounds, sides, virt
    SWAP1  
    SWAP2 
    // stack:        a, x, box, b, c, d, e, F, K, boxes, rounds, sides, virt
    ADD  
    DUP8  
    ADD  
    %u32
    // stack:           a, box, b, c, d, e, F, K, boxes, rounds, sides, virt
    PUSH mid_rol  
    SWAP2
    // stack:  box, a, mid_rol, b, c, d, e, F, K, boxes, rounds, sides, virt
    %mload_kernel_code_label(S_data)
    // stack:    s, a, mid_rol, b, c, d, e, F, K, boxes, rounds, sides, virt
    %jump(rol)
mid_rol:
    // stack:               a, b, c, d, e, F, K, boxes, rounds, sides, virt
    DUP5
    // stack:            e, a, b, c, d, e, F, K, boxes, rounds, sides, virt
    ADD 
    %u32    
    // stack:               a, b, c, d, e, F, K, boxes, rounds, sides, virt
    SWAP1  
    SWAP2  
    PUSH post_rol  
    SWAP1  
    PUSH 10
    // stack: 10, c, post_rol, b, a, d, e, F, K, boxes, rounds, sides, virt
    %jump(rol)
post_rol:
    // stack: c, a, b, d, e, F, K, boxes  , rounds, sides, virt
    SWAP3
    // stack: d, a, b, c, e, F, K, boxes  , rounds, sides, virt
    SWAP4
    // stack: e, a, b, c, d, F, K, boxes  , rounds, sides, virt
    SWAP7  
    PUSH 1  
    SWAP1  
    SUB  
    SWAP7
    // stack: e, a, b, c, d, F, K, boxes-1, rounds, sides, virt
    %jump(round)


%macro get_round
    // stack: sides, rounds
    %mul_const(5)  
    PUSH 10  
    SUB  
    SUB
    // stack: 10 - 5*sides - rounds
%endmacro

%macro get_box
    // stack:                                     ARGS: 7, boxes, rounds, sides
    DUP10  
    %mul_const(80)  
    DUP10  
    %mul_const(16)  
    DUP10  
    // stack:       boxes , 16*rounds , 80*sides, ARGS: 7, boxes, rounds, sides
    PUSH 176  
    SUB  
    SUB  
    SUB
    // stack: 176 - boxes - 16*rounds - 80*sides, ARGS: 7, boxes, rounds, sides
%endmacro
