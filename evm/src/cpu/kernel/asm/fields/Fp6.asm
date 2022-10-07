// cost: 6 loads + 6 offsets + 5 adds = 6*4 + 6*1 + 5*2 = 40
%macro load_fp6(offset)
    // stack:
    PUSH $offset
    %add_const(5)
    %mload_kernel_general
    // stack:                     x5
    PUSH $offset
    %add_const(4)
    %mload_kernel_general
    // stack:                 x4, x5
    PUSH $offset
    %add_const(3)
    %mload_kernel_general
    // stack:             x3, x4, x5
    PUSH $offset
    %add_const(2)
    %mload_kernel_general
    // stack:         x2, x3, x4, x5
    PUSH $offset
    %add_const(1)
    %mload_kernel_general
    // stack:     x1, x2, x3, x4, x5
    PUSH $offset
    %mload_kernel_general
    // stack: x0, x1, x2, x3, x4, x5
%endmacro

// cost: 40
%macro store_fp6(offset)
    // stack: x0, x1, x2, x3, x4, x5
    PUSH $offset
    %mstore_kernel_general
    // stack:     x1, x2, x3, x4, x5
    PUSH $offset
    %add_const(1)
    %mstore_kernel_general
    // stack:         x2, x3, x4, x5
    PUSH $offset
    %add_const(2)
    %mstore_kernel_general
    // stack:             x3, x4, x5
    PUSH $offset
    %add_const(3)
    %mstore_kernel_general
    // stack:                 x4, x5
    PUSH $offset
    %add_const(4)
    %mstore_kernel_general
    // stack:                     x5
    PUSH $offset
    %add_const(5)
    %mstore_kernel_general
    // stack:
%endmacro

// cost: 6
%macro dup1_fp6
    // stack:       F: 6
    DUP6
    DUP6
    DUP6
    DUP6
    DUP6
    DUP6
    // stack: F: 6, F: 6
%endmacro 

// cost: 6
%macro dup2_fp6
    // stack:       F: 6, G: 6
    DUP12
    DUP12
    DUP12
    DUP12
    DUP12
    DUP12
    // stack: G: 6, F: 6, G: 6
%endmacro

// cost: 16
%macro swap_fp6
    // stack: f0, f1, f2, f3, f4, f5, g0, g1, g2, g3, g4, g5
    SWAP6
    // stack: g0, f1, f2, f3, f4, f5, f0, g1, g2, g3, g4, g5
    SWAP1
    SWAP7
    SWAP1
    // stack: g0, g1, f2, f3, f4, f5, f0, f1, g2, g3, g4, g5
    SWAP2
    SWAP8
    SWAP2
    // stack: g0, g1, g2, f3, f4, f5, f0, f1, f2, g3, g4, g5
    SWAP3
    SWAP9
    SWAP3    
    // stack: g0, g1, g2, g3, f4, f5, f0, f1, f2, f3, g4, g5
    SWAP4
    SWAP10
    SWAP4
    // stack: g0, g1, g2, g3, g4, f5, f0, f1, f2, f3, f4, g5
    SWAP5
    SWAP11
    SWAP5
    // stack: g0, g1, g2, g3, g4, g5, f0, f1, f2, f3, f4, f5
%endmacro

// cost: 16
%macro add_fp6
    // stack: f0, f1, f2, f3, f4, f5, g0, g1, g2, g3, g4, g5
    SWAP7
    ADD
    SWAP6
    // stack: f0,     f2, f3, f4, f5, g0, h1, g2, g3, g4, g5 
    SWAP7
    ADD
    SWAP6
    // stack: f0,         f3, f4, f5, g0, h1, h2, g3, g4, g5 
    SWAP7
    ADD
    SWAP6
    // stack: f0,             f4, f5, g0, h1, h2, h3, g4, g5
    SWAP7
    ADD
    SWAP6
    // stack: f0,                 f5, g0, h1, h2, h3, h4, g5
    SWAP7
    ADD
    SWAP6
    // stack: f0,                     g0, h1, h2, h3, h4, h5
    ADD
    // stack:                         h0, h1, h2, h3, h4, h5
%endmacro

// *backwards order subtraction* cost: 16
%macro bus_fp6
    // stack: f0, f1, f2, f3, f4, f5, g0, g1, g2, g3, g4, g5
    SWAP7
    SUB
    SWAP6
    // stack: f0,     f2, f3, f4, f5, g0, h1, g2, g3, g4, g5 
    SWAP7
    SUB
    SWAP6
    // stack: f0,         f3, f4, f5, g0, h1, h2, g3, g4, g5 
    SWAP7
    SUB
    SWAP6
    // stack: f0,             f4, f5, g0, h1, h2, h3, g4, g5
    SWAP7
    SUB
    SWAP6
    // stack: f0,                 f5, g0, h1, h2, h3, h4, g5
    SWAP7
    SUB
    SWAP6
    // stack: f0,                     g0, h1, h2, h3, h4, h5
    SUB
    // stack:                         h0, h1, h2, h3, h4, h5
%endmacro

%macro mul_Fp6
    DUP3
    DUP11
    MUL
    DUP2
    DUP14
    MUL
    SUB
    DUP12
    DUP8
    MUL
    DUP9
    DUP5
    MUL
    ADD
    DUP7
    DUP7
    MUL
    DUP12
    DUP12
    MUL
    ADD
    SUB
    DUP7
    DUP5
    MUL
    DUP10
    DUP8
    MUL
    ADD
    DUP12
    DUP10
    MUL
    ADD
    DUP14
    DUP12
    MUL
    ADD
    DUP15
    DUP7
    MUL
    DUP14
    DUP6
    MUL
    ADD
    DUP2
    DUP4
    PUSH 9
    MUL
    SUB
    ADD
    %mstore_kernel_general(0)
    PUSH 9
    MUL
    ADD
    ADD
    %mstore_kernel_general(1)
    DUP4
    DUP12
    MUL
    DUP3
    DUP11
    MUL
    ADD
    DUP3
    DUP13
    MUL
    DUP6
    DUP12
    MUL
    SUB
    DUP12
    DUP10
    MUL
    DUP9
    DUP7
    MUL
    ADD
    DUP2
    DUP4
    PUSH 9
    MUL
    SUB
    SUB
    DUP15
    DUP9
    MUL
    DUP12
    DUP6
    MUL
    ADD
    ADD
    %mstore_kernel_general(2)
    PUSH 9
    MUL
    ADD
    DUP13
    DUP9
    MUL
    DUP12
    DUP8
    MUL
    ADD
    DUP10
    DUP6
    MUL
    ADD
    DUP8
    DUP4
    MUL
    ADD
    ADD
    %mstore_kernel_general(3)
    DUP10
    DUP12
    MUL
    DUP7
    DUP9
    MUL
    ADD
    DUP3
    DUP5
    MUL
    ADD
    DUP13
    DUP11
    MUL
    DUP10
    DUP8
    MUL
    ADD
    DUP6
    DUP4
    MUL
    ADD
    SUB
    %mstore_kernel_general(4)
    MUL
    SWAP2
    MUL
    ADD
    SWAP2
    MUL
    ADD
    SWAP2
    MUL
    ADD
    SWAP2
    MUL
    ADD
    SWAP2
    MUL
    ADD
    %mstore_kernel_general(5)
%endmacro
