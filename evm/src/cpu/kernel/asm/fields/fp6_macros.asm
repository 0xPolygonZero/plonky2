// cost: 6 loads + 6 dup/swaps + 5 adds = 6*4 + 6*1 + 5*2 = 40
%macro load_fp6
    // stack: ptr
    DUP1  %add_const(4)
    // stack:                   ind4, ptr
    %mload_kernel_general
    // stack:                     x4, ptr
    DUP2  %add_const(3)
    // stack:               ind3, x4, ptr
    %mload_kernel_general
    // stack:                 x3, x4, ptr
    DUP3  %add_const(2)
    // stack:           ind2, x3, x4, ptr
    %mload_kernel_general
    // stack:             x2, x3, x4, ptr
    DUP4  %add_const(1)
    // stack:       ind1, x2, x3, x4, ptr
    %mload_kernel_general
    // stack:         x1, x2, x3, x4, ptr
    DUP5  %add_const(5)
    // stack:   ind5, x1, x2, x3, x4, ptr
    %mload_kernel_general
    // stack:     x5, x1, x2, x3, x4, ptr
    SWAP5
    // stack:   ind0, x1, x2, x3, x4, x5
    %mload_kernel_general
    // stack:     x0, x1, x2, x3, x4, x5
%endmacro

// cost: 6 loads + 6 pushes + 5 adds = 6*4 + 6*1 + 5*2 = 40
%macro load_fp6(ptr)
    // stack:
    PUSH $ptr  %add_const(5)
    // stack:                     ind5
    %mload_kernel_general
    // stack:                       x5
    PUSH $ptr  %add_const(4)
    // stack:                 ind4, x5
    %mload_kernel_general
    // stack:                   x4, x5
    PUSH $ptr  %add_const(3)
    // stack:             ind3, x4, x5
    %mload_kernel_general
    // stack:               x3, x4, x5
    PUSH $ptr  %add_const(2)
    // stack:         ind2, x3, x4, x5
    %mload_kernel_general
    // stack:           x2, x3, x4, x5
    PUSH $ptr  %add_const(1)
    // stack:     ind1, x2, x3, x4, x5
    %mload_kernel_general
    // stack:       x1, x2, x3, x4, x5
    PUSH $ptr
    // stack: ind0, x1, x2, x3, x4, x5
    %mload_kernel_general
    // stack:   x0, x1, x2, x3, x4, x5
%endmacro

// cost: 6 stores + 6 swaps/dups + 5 adds = 6*4 + 6*1 + 5*2 = 40
%macro store_fp6
    // stack:      ptr, x0, x1, x2, x3, x4 , x5
    SWAP5
    // stack:       x4, x0, x1, x2, x3, ptr, x5
    DUP6  %add_const(4)
    // stack: ind4, x4, x0, x1, x2, x3, ptr, x5
    %mstore_kernel_general
    // stack:           x0, x1, x2, x3, ptr, x5
    DUP5
    // stack:     ind0, x0, x1, x2, x3, ptr, x5
    %mstore_kernel_general
    // stack:               x1, x2, x3, ptr, x5
    DUP4  %add_const(1)
    // stack:         ind1, x1, x2, x3, ptr, x5
    %mstore_kernel_general
    // stack:                   x2, x3, ptr, x5
    DUP3  %add_const(2)
    // stack:             ind2, x2, x3, ptr, x5
    %mstore_kernel_general
    // stack:                       x3, ptr, x5
    DUP2  %add_const(3)
    // stack:                 ind3, x3, ptr, x5
    %mstore_kernel_general
    // stack:                           ptr, x5
    %add_const(5)
    // stack:                          ind5, x5
    %mstore_kernel_general
    // stack:
%endmacro

// cost: 6 stores + 6 pushes + 5 adds = 6*4 + 6*1 + 5*2 = 40
%macro store_fp6(ptr)
    // stack:       x0, x1, x2, x3, x4, x5
    PUSH $ptr
    // stack: ind0, x0, x1, x2, x3, x4, x5
    %mstore_kernel_general
    // stack:           x1, x2, x3, x4, x5
    PUSH $ptr  %add_const(1)
    // stack:     ind1, x1, x2, x3, x4, x5
    %mstore_kernel_general
    // stack:               x2, x3, x4, x5
    PUSH $ptr  %add_const(2)
    // stack:         ind2, x2, x3, x4, x5
    %mstore_kernel_general
    // stack:                   x3, x4, x5
    PUSH $ptr  %add_const(3)
    // stack:             ind3, x3, x4, x5
    %mstore_kernel_general
    // stack:                       x4, x5
    PUSH $ptr  %add_const(4)
    // stack:                 ind4, x4, x5
    %mstore_kernel_general
    // stack:                           x5
    PUSH $ptr  %add_const(5)
    // stack:                     ind5, x5
    %mstore_kernel_general
    // stack:
%endmacro

// cost: store (40) + i9 (9) = 49
%macro store_fp6_sh(ptr)
    // stack:       x0, x1, x2, x3, x4, x5
    PUSH $ptr  %add_const(2)
    // stack: ind2, x0, x1, x2, x3, x4, x5
    %mstore_kernel_general
    // stack:           x1, x2, x3, x4, x5
    PUSH $ptr  %add_const(3)
    // stack:     ind3, x1, x2, x3, x4, x5
    %mstore_kernel_general
    // stack:               x2, x3, x4, x5
    PUSH $ptr  %add_const(4)
    // stack:         ind4, x2, x3, x4, x5
    %mstore_kernel_general
    // stack:                   x3, x4, x5
    PUSH $ptr  %add_const(5)
    // stack:             ind5, x3, x4, x5
    %mstore_kernel_general
    // stack:                       x4, x5
    %i9
    // stack:                       y5, y4
    PUSH $ptr  %add_const(1)
    // stack:                 ind1, y5, y4
    %mstore_kernel_general
    // stack:                           y4
    PUSH $ptr
    // stack:                     ind0, y4
    %mstore_kernel_general
    // stack:
%endmacro

// cost: 9; note this returns y, x for the output x + yi
%macro i9
    // stack:          a , b
    DUP2
    // stack:      b,  a,  b
    DUP2
    // stack:  a , b,  a , b
    PUSH 9  MULFP254
    // stack: 9a , b,  a , b
    SUBFP254
    // stack: 9a - b,  a , b
    SWAP2 
    // stack:  b , a, 9a - b
    PUSH 9  MULFP254
    // stack  9b , a, 9a - b
    ADDFP254
    // stack: 9b + a, 9a - b 
%endmacro

// cost: 6
%macro dup_fp6_0
    // stack:       f: 6
    DUP6
    DUP6
    DUP6
    DUP6
    DUP6
    DUP6
    // stack: f: 6, g: 6
%endmacro 

// cost: 6
%macro dup_fp6_6
    // stack:       f: 6, g: 6
    DUP12
    DUP12
    DUP12
    DUP12
    DUP12
    DUP12
    // stack: g: 6, f: 6, g: 6
%endmacro

// cost: 6
%macro dup_fp6_7
    // stack:       f: 6, g: 6
    DUP13
    DUP13
    DUP13
    DUP13
    DUP13
    DUP13
    // stack: g: 6, f: 6, g: 6
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
// swap two fp6 elements with a stack term separating them
//    (f: 6, x, g: 6) -> (g: 6, x, f: 6)
%macro swap_fp6_hole
    // stack: f0, f1, f2, f3, f4, f5, X, g0, g1, g2, g3, g4, g5
    SWAP7
    // stack: g0, f1, f2, f3, f4, f5, X, f0, g1, g2, g3, g4, g5
    SWAP1
    SWAP8
    SWAP1
    // stack: g0, g1, f2, f3, f4, f5, X, f0, f1, g2, g3, g4, g5
    SWAP2
    SWAP9
    SWAP2
    // stack: g0, g1, g2, f3, f4, f5, X, f0, f1, f2, g3, g4, g5
    SWAP3
    SWAP10
    SWAP3    
    // stack: g0, g1, g2, g3, f4, f5, X, f0, f1, f2, f3, g4, g5
    SWAP4
    SWAP11
    SWAP4
    // stack: g0, g1, g2, g3, g4, f5, X, f0, f1, f2, f3, f4, g5
    SWAP5
    SWAP12
    SWAP5
    // stack: g0, g1, g2, g3, g4, g5, X, f0, f1, f2, f3, f4, f5
%endmacro

// cost: 16
%macro add_fp6
    // stack: f0, f1, f2, f3, f4, f5, g0, g1, g2, g3, g4, g5
    SWAP7
    ADDFP254
    SWAP6
    // stack: f0,     f2, f3, f4, f5, g0, h1, g2, g3, g4, g5 
    SWAP7
    ADDFP254
    SWAP6
    // stack: f0,         f3, f4, f5, g0, h1, h2, g3, g4, g5 
    SWAP7
    ADDFP254
    SWAP6
    // stack: f0,             f4, f5, g0, h1, h2, h3, g4, g5
    SWAP7
    ADDFP254
    SWAP6
    // stack: f0,                 f5, g0, h1, h2, h3, h4, g5
    SWAP7
    ADDFP254
    SWAP6
    // stack: f0,                     g0, h1, h2, h3, h4, h5
    ADDFP254
    // stack:                         h0, h1, h2, h3, h4, h5
%endmacro

// *reversed argument subtraction* cost: 17
%macro subr_fp6
    // stack: f0, f1, f2, f3, f4, f5, g0, g1, g2, g3, g4, g5
    SWAP7
    SUBFP254
    SWAP6
    // stack: f0,     f2, f3, f4, f5, g0, h1, g2, g3, g4, g5 
    SWAP7
    SUBFP254
    SWAP6
    // stack: f0,         f3, f4, f5, g0, h1, h2, g3, g4, g5 
    SWAP7
    SUBFP254
    SWAP6
    // stack: f0,             f4, f5, g0, h1, h2, h3, g4, g5
    SWAP7
    SUBFP254
    SWAP6
    // stack: f0,                 f5, g0, h1, h2, h3, h4, g5
    SWAP7
    SUBFP254
    SWAP6
    // stack: f0,                     g0, h1, h2, h3, h4, h5
    SWAP1
    SUBFP254
    // stack:                         h0, h1, h2, h3, h4, h5
%endmacro
