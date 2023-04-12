/// Note that we unpack STATE: 5 to a, b, c, d, e 
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

global box:
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
    %mload_kernel_code(r_data)
    ADD
    // stack: virt + r, box, a, b, c, d, e, F, K, boxes, rounds, sides, virt  
    %mload_kernel_general_u32_LE
    // stack:        x, box, a, b, c, d, e, F, K, boxes, rounds, sides, virt
    SWAP1  
    SWAP2 
    // stack:        a, x, box, b, c, d, e, F, K, boxes, rounds, sides, virt
    ADD  
    DUP8  
    ADD  
    %as_u32
    // stack:           a, box, b, c, d, e, F, K, boxes, rounds, sides, virt
    PUSH mid_rol  
    SWAP2
    // stack:  box, a, mid_rol, b, c, d, e, F, K, boxes, rounds, sides, virt
    %mload_kernel_code(s_data)
    // stack:    s, a, mid_rol, b, c, d, e, F, K, boxes, rounds, sides, virt
    %jump(rol)
mid_rol:
    // stack:               a, b, c, d, e, F, K, boxes, rounds, sides, virt
    DUP5
    // stack:            e, a, b, c, d, e, F, K, boxes, rounds, sides, virt
    ADD 
    %as_u32    
    // stack:               a, b, c, d, e, F, K, boxes, rounds, sides, virt
    %stack (a, b, c) -> (10, c, post_rol, a, b) 
    // stack: 10, c, post_rol, a, b, d, e, F, K, boxes, rounds, sides, virt
    %jump(rol)
post_rol:
    // stack: c, a, b, d, e, F, K, boxes  , rounds, sides, virt
    %stack (c, a, b, d, e, F, K, boxes) -> (boxes, 1, a, b, c, d, F, K, e)
    // stack: boxes, 1, a, b, c, d, F, K, e, rounds, sides, virt
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
