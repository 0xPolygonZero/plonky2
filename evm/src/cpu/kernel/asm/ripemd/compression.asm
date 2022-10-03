/// _block is stored in memory: its address virt stays on the stack
/// def compress(STATE: 5, _block):
/// 
///     STATEL = STATE
///     STATEL = loop(STATEL)
///
///     STATER = state
///     STATER = loop(STATER)
///
///     return mix(STATER, STATEL, STATE)
///
///
/// def mix(STATER, STATEL, STATE):
///     return
///         u32(s1 + l2 + r3),
///         u32(s2 + l3 + r4),
///         u32(s3 + l4 + r0),
///         u32(s4 + l0 + r1),
///         u32(s0 + l1 + r2)
/// 
/// where si, li, ri, oi, VR, RD respectively denote 
/// STATE[i], STATEL[i], STATER[i], OUTPUT[i], virt, retdest

global compress:
    // stack:                                       STATE, virt, retdest
    PUSH switch
    DUP7
    %stack () ->     (0, 0, 16, 5, 1)
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
    %stack () ->          (16, 5, 0)
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


/// def loop(STATE: 5):
///     while rounds:
///         update_round_vars()
///         round(STATE: 5, F, K, rounds, sides)
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
    %mload_kernel_code_u32(k_data)
    SWAP7  
    POP
    // stack:           STATE, F', K', 16, rounds, sides, virt, retdest
    %jump(round)
global round:
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
