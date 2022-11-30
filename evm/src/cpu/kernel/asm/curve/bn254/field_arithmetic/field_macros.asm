%macro offset_fp6
    %add_const(6)
%endmacro

// fp2 macros

// cost: 2 loads + 6 dup/swaps + 5 adds = 6*4 + 6*1 + 5*2 = 40
%macro load_fp2
    // stack:       ptr
    DUP1  %add_const(1)
    // stack: ind1, ptr
    %mload_kernel_general
    // stack:   x1, ptr
    SWAP1
    // stack: ind0, x1
    %mload_kernel_general
    // stack:   x0, x1
%endmacro 

%macro conj
    // stack: a,  b
    SWAP1 
    PUSH 0
    SUBFP254
    SWAP1
    // stack: a, -b 
%endmacro

%macro swap_fp2
    // stack: a , a_, b , b_
    SWAP2
    // stack: b , a_, a , b_
    SWAP1
    // stack: a_, b , a , b_
    SWAP3
    // stack: b_, b , a , a_
    SWAP1 
    // stack: b , b_, a , a_
%endmacro

%macro swap_fp2_hole_2
    // stack: a , a_, X, b , b_
    SWAP4
    // stack: b , a_, X, a , b_
    SWAP1
    // stack: a_, b , X, a , b_
    SWAP5
    // stack: b_, b , X, a , a_
    SWAP1 
    // stack: b , b_, X, a , a_
%endmacro

%macro mul_fp_fp2
    // stack:    c, x, y
    SWAP2
    // stack:    y, x, c 
    DUP3
    // stack: c, y, x, c
    MULFP254
    // stack:   cy, x, c
    SWAP2
    // stack:   c, x, cy
    MULFP254
    // stack:     cx, cy 
%endmacro

// cost: 9; note this returns y, x for the output x + yi
%macro i9
    // stack:          a , b
    DUP2
    // stack:      b,  a , b
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

%macro mul_fp2
    // stack:          a, b, c, d
    DUP4
    DUP3
    MULFP254
    // stack:      bd, a, b, c, d
    DUP4 
    DUP3
    MULFP254
    // stack: ac , bd, a, b, c, d 
    SUBFP254
    // stack: ac - bd, a, b, c, d 
    SWAP4
    // stack: d, a, b, c, ac - bd
    MULFP254
    // stack:   ad, b, c, ac - bd
    SWAP2
    // stack:   c, b, ad, ac - bd
    MULFP254
    // stack:    bc , ad, ac - bd
    ADDFP254
    // stack:    bc + ad, ac - bd
    SWAP1
    // stack:    ac - bd, bc + ad
%endmacro 

// fp6 macros

// cost: 6 loads + 6 dup/swaps + 5 adds = 6*4 + 6*1 + 5*2 = 40
%macro load_fp6
    // stack:                         ptr
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

// cost: 6 stores + 7 swaps/dups + 5 adds + 6 doubles = 6*4 + 7*1 + 5*2 + 6*2 = 53
%macro store_fp6_double
    // stack:        ptr, x0, x1, x2, x3, x4, x5
    SWAP6
    // stack:         x5, x0, x1, x2, x3, x4, ptr
    PUSH 2  MULFP254
    // stack:       2*x5, x0, x1, x2, x3, x4, ptr
    DUP7  %add_const(5)
    // stack: ind5, 2*x5, x0, x1, x2, x3, x4, ptr
    %mstore_kernel_general
    // stack:             x0, x1, x2, x3, x4, ptr
    PUSH 2  MULFP254
    // stack:           2*x0, x1, x2, x3, x4, ptr
    DUP6
    // stack:     ind0, 2*x0, x1, x2, x3, x4, ptr
    %mstore_kernel_general
    // stack:                 x1, x2, x3, x4, ptr
    PUSH 2  MULFP254
    // stack:               2*x1, x2, x3, x4, ptr
    DUP5  %add_const(1)
    // stack:         ind1, 2*x1, x2, x3, x4, ptr
    %mstore_kernel_general
    // stack:                     x2, x3, x4, ptr
    PUSH 2  MULFP254
    // stack:                   2*x2, x3, x4, ptr
    DUP4  %add_const(2)
    // stack:             ind2, 2*x2, x3, x4, ptr
    %mstore_kernel_general
    // stack:                         x3, x4, ptr
    PUSH 2  MULFP254
    // stack:                       2*x3, x4, ptr
    DUP3  %add_const(3)
    // stack:                 ind3, 2*x3, x4, ptr
    %mstore_kernel_general
    // stack:                             x4, ptr
    PUSH 2  MULFP254
    // stack:                           2*x4, ptr
    SWAP1
    // stack:                           ptr, 2*x4
    %add_const(4)
    // stack:                          ind4, 2*x4
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

// cost: 6
%macro dup_fp6_0
    // stack:       f: 6
    DUP6
    DUP6
    DUP6
    DUP6
    DUP6
    DUP6
    // stack: f: 6, f: 6
%endmacro 

// cost: 6
%macro dup_fp6_2
    // stack:       X: 2, f: 6
    DUP8
    DUP8
    DUP8
    DUP8
    DUP8
    DUP8
    // stack: f: 6, X: 2, f: 6
%endmacro 

// cost: 6
%macro dup_fp6_6
    // stack:       X: 6, f: 6
    DUP12
    DUP12
    DUP12
    DUP12
    DUP12
    DUP12
    // stack: f: 6, X: 6, f: 6
%endmacro

// cost: 6
%macro dup_fp6_7
    // stack:       X: 7, f: 6
    DUP13
    DUP13
    DUP13
    DUP13
    DUP13
    DUP13
    // stack: f: 6, X: 7, f: 6
%endmacro

// cost: 6
%macro dup_fp6_8
    // stack:       X: 8, f: 6
    DUP14
    DUP14
    DUP14
    DUP14
    DUP14
    DUP14
    // stack: f: 6, X: 8, f: 6
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
//    (f: 6, X, g: 6) -> (g: 6, X, f: 6)
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
// swap two fp6 elements with two stack terms separating them
//    (f: 6, X: 2, g: 6) -> (g: 6, X: 2, f: 6)
%macro swap_fp6_hole_2
    // stack: f0, f1, f2, f3, f4, f5, X, g0, g1, g2, g3, g4, g5
    SWAP8
    // stack: g0, f1, f2, f3, f4, f5, X, f0, g1, g2, g3, g4, g5
    SWAP1
    SWAP9
    SWAP1
    // stack: g0, g1, f2, f3, f4, f5, X, f0, f1, g2, g3, g4, g5
    SWAP2
    SWAP10
    SWAP2
    // stack: g0, g1, g2, f3, f4, f5, X, f0, f1, f2, g3, g4, g5
    SWAP3
    SWAP11
    SWAP3    
    // stack: g0, g1, g2, g3, f4, f5, X, f0, f1, f2, f3, g4, g5
    SWAP4
    SWAP12
    SWAP4
    // stack: g0, g1, g2, g3, g4, f5, X, f0, f1, f2, f3, f4, g5
    SWAP5
    SWAP13
    SWAP5
    // stack: g0, g1, g2, g3, g4, g5, X, f0, f1, f2, f3, f4, f5
%endmacro

%macro sh
    // stack: f0 , f0_, f1,  f1_, f2 , f2_
    SWAP2
    // stack: f1 , f0_, g0 , f1_, f2 , f2_
    SWAP4
    // stack: f2 , f0_, g0 , f1_, g1 , f2_
    SWAP1
    // stack: f0_, f2 , g0 , f1_, g1 , f2_
    SWAP3
    // stack: f1_, f2 , g0 , g0_, g1 , f2_
    SWAP5
    // stack: f2_, f2 , g0 , g0_, g1 , g1_
    SWAP1 
    // stack: f2 , f2_, g0 , g0_, g1 , g1_
    %i9
    // stack: g2_, g2 , g0 , g0_, g1 , g1_
    SWAP1
    // stack: g2 , g2_, g0 , g0_, g1 , g1_
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

// cost: 18
// add two fp6 elements with a to-be-popped stack term separating them
//    (f: 6, X, g: 6) -> (f + g: 6)
%macro add_fp6_hole
    // stack: f0, f1, f2, f3, f4, f5, X, g0, g1, g2, g3, g4, g5
    SWAP8
    ADDFP254
    SWAP7
    // stack: f0,     f2, f3, f4, f5, X, g0, h1, g2, g3, g4, g5 
    SWAP8
    ADDFP254
    SWAP7
    // stack: f0,         f3, f4, f5, X, g0, h1, h2, g3, g4, g5 
    SWAP8
    ADDFP254
    SWAP7
    // stack: f0,             f4, f5, X, g0, h1, h2, h3, g4, g5
    SWAP8
    ADDFP254
    SWAP7
    // stack: f0,                 f5, X, g0, h1, h2, h3, h4, g5
    SWAP8
    ADDFP254
    SWAP7
    // stack: f0,                     X, g0, h1, h2, h3, h4, h5
    SWAP1
    POP
    ADDFP254
    // stack:                            h0, h1, h2, h3, h4, h5
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

// cost: 21
%macro mul_fp_fp6
    // stack: c , f0,      f1,    f2,     f3,     f4,     f5
    SWAP6
    DUP7
    MULFP254
    SWAP6
    // stack: c , f0,      f1,    f2,     f3,     f4, c * f5
    SWAP5
    DUP6
    MULFP254
    SWAP5
    // stack: c , f0,     f1,     f2,     f3, c * f4, c * f5
    SWAP4
    DUP5
    MULFP254
    SWAP4
    // stack: c , f0,     f1,     f2, c * f3, c * f4, c * f5
    SWAP3 
    DUP4 
    MULFP254
    SWAP3 
    // stack: c , f0,     f1, c * f2, c * f3, c *f 4, c * f5
    SWAP2  
    DUP3  
    MULFP254
    SWAP2  
    // stack: c , f0, c * f1, c * f2, c * f3, c * f4, c * f5
    MULFP254
    // stack: c * f0, c * f1, c * f2, c * f3, c * f4, c * f5
%endmacro

/// cost: 
///
/// G0 + G1t + G2t^2 = (a+bi) * (F0 + F1t + F2t^2) 
///                  = (a+bi)F0 + (a+bi)F1t + (a+bi)F2t^2
///
/// G0 = (a+bi)(f0+f0_i) = (af0 - bf0_) + (bf0 + af0_)i
/// G1 = (a+bi)(f1+f1_i) = (af1 - bf1_) + (bf1 + af1_)i
/// G2 = (a+bi)(f2+f2_i) = (af2 - bf2_) + (bf2 + af2_)i

%macro mul_fp2_fp6
    // stack:             a, b, f0, f0_, f1, f1_, f2, f2_
    DUP2
    DUP5
    MULFP254
    // stack:       bf0_, a, b, f0, f0_, f1, f1_, f2, f2_
    DUP2
    DUP5
    MULFP254
    // stack:  af0, bf0_, a, b, f0, f0_, f1, f1_, f2, f2_
    SUBFP254
    // stack:         g0, a, b, f0, f0_, f1, f1_, f2, f2_
    SWAP3
    // stack:         f0, a, b, g0, f0_, f1, f1_, f2, f2_
    DUP3
    MULFP254
    // stack:        bf0, a, b, g0, f0_, f1, f1_, f2, f2_
    SWAP1
    SWAP4
    // stack:        f0_, bf0, b, g0, a, f1, f1_, f2, f2_
    DUP5
    MULFP254
    // stack:       af0_, bf0, b, g0, a, f1, f1_, f2, f2_
    ADDFP254
    // stack:             g0_, b, g0, a, f1, f1_, f2, f2_
    SWAP3
    // stack:             a, b, g0, g0_, f1, f1_, f2, f2_
    DUP2
    DUP7
    MULFP254
    // stack:       bf1_, a, b, g0, g0_, f1, f1_, f2, f2_
    DUP2
    DUP7
    MULFP254
    // stack:  af1, bf1_, a, b, g0, g0_, f1, f1_, f2, f2_
    SUBFP254
    // stack:         g1, a, b, g0, g0_, f1, f1_, f2, f2_
    SWAP5
    // stack:         f1, a, b, g0, g0_, g1, f1_, f2, f2_
    DUP3
    MULFP254
    // stack:        bf1, a, b, g0, g0_, g1, f1_, f2, f2_
    SWAP1
    SWAP6
    // stack:        f1_, bf1, b, g0, g0_, g1, a, f2, f2_
    DUP7
    MULFP254
    // stack:       af1_, bf1, b, g0, g0_, g1, a, f2, f2_
    ADDFP254
    // stack:             g1_, b, g0, g0_, g1, a, f2, f2_
    SWAP5
    // stack:             a, b, g0, g0_, g1, g1_, f2, f2_
    DUP2
    DUP9
    MULFP254
    // stack:       bf2_, a, b, g0, g0_, g1, g1_, f2, f2_
    DUP2
    DUP9
    MULFP254
    // stack:  af2, bf2_, a, b, g0, g0_, g1, g1_, f2, f2_
    SUBFP254
    // stack:         g2, a, b, g0, g0_, g1, g1_, f2, f2_
    SWAP7
    // stack:         f2, a, b, g0, g0_, g1, g1_, g2, f2_
    SWAP8
    // stack:         f2_, a, b, g0, g0_, g1, g1_, g2, f2
    MULFP254
    // stack:           af2_, b, g0, g0_, g1, g1_, g2, f2
    SWAP7
    // stack:           f2, b, g0, g0_, g1, g1_, g2, af2_
    MULFP254
    // stack:             bf2, g0, g0_, g1, g1_, g2, af2_
    SWAP1
    SWAP6
    // stack:             af2_, bf2, g0_, g1, g1_, g2, g0
    ADDFP254
    // stack:                   g2_, g0_, g1, g1_, g2, g0
    SWAP5
    // stack:                   g0, g0_, g1, g1_, g2, g2_
%endmacro 

/// cost: 1 i9 (9) + 16 dups + 15 swaps + 12 muls + 6 adds/subs = 58
///
/// G0 + G1t + G2t^2 = (a+bi)t * (F0 + F1t + F2t^2) 
///                  = (c+di)F2 + (a+bi)F0t + (a+bi)F1t^2
/// where c+di = (a+bi)(9+i) = (9a-b) + (a+9b)i 
///
/// G0 = (c+di)(f2+f2_i) = (cf2 - df2_) + (df2 + cf2_)i
/// G1 = (a+bi)(f0+f0_i) = (af0 - bf0_) + (bf0 + af0_)i
/// G2 = (a+bi)(f1+f1_i) = (af1 - bf1_) + (bf1 + af1_)i

%macro mul_fp2_fp6_sh
    // stack:             a, b, f0, f0_, f1, f1_, f2, f2_
    DUP6
    DUP3
    MULFP254
    // stack:       bf1_, a, b, f0, f0_, f1, f1_, f2, f2_
    DUP6 
    DUP3
    MULFP254
    // stack: af1 , bf1_, a, b, f0, f0_, f1, f1_, f2, f2_
    SUBFP254
    // stack:         g2, a, b, f0, f0_, f1, f1_, f2, f2_
    SWAP7
    // stack:         f2, a, b, f0, f0_, f1, f1_, g2, f2_
    SWAP5
    // stack:         f1, a, b, f0, f0_, f2, f1_, g2, f2_
    DUP3
    MULFP254
    // stack:        bf1, a, b, f0, f0_, f2, f1_, g2, f2_
    SWAP1
    SWAP6
    // stack:        f1_, bf1, b, f0, f0_, f2, a, g2, f2_
    DUP7
    MULFP254
    // stack:       af1_, bf1, b, f0, f0_, f2, a, g2, f2_
    ADDFP254
    // stack:             g2_, b, f0, f0_, f2, a, g2, f2_
    SWAP7
    // stack:             f2_, b, f0, f0_, f2, a, g2, g2_
    DUP4
    DUP3
    MULFP254
    // stack:       bf0_, f2_, b, f0, f0_, f2, a, g2, g2_
    DUP4
    DUP8
    MULFP254
    // stack:  af0, bf0_, f2_, b, f0, f0_, f2, a, g2, g2_
    SUBFP254 
    // stack:         g1, f2_, b, f0, f0_, f2, a, g2, g2_
    SWAP5
    // stack:         f2, f2_, b, f0, f0_, g1, a, g2, g2_
    SWAP3
    // stack:         f0, f2_, b, f2, f0_, g1, a, g2, g2_
    DUP3
    MULFP254
    // stack:        bf0, f2_, b, f2, f0_, g1, a, g2, g2_
    SWAP1
    SWAP4
    // stack:        f0_, bf0, b, f2, f2_, g1, a, g2, g2_
    DUP7
    MULFP254
    // stack:       af0_, bf0, b, f2, f2_, g1, a, g2, g2_
    ADDFP254
    // stack:             g1_, b, f2, f2_, g1, a, g2, g2_
    SWAP5 
    // stack:             a, b, f2, f2_, g1, g1_, g2, g2_
    %i9
    // stack:             d, c, f2, f2_, g1, g1_, g2, g2_
    DUP4
    DUP2
    MULFP254
    // stack:       df2_, d, c, f2, f2_, g1, g1_, g2, g2_
    DUP4
    DUP4
    MULFP254
    // stack:  cf2, df2_, d, c, f2, f2_, g1, g1_, g2, g2_
    SUBFP254
    // stack:         g0, d, c, f2, f2_, g1, g1_, g2, g2_
    SWAP3 
    // stack:         f2, d, c, g0, f2_, g1, g1_, g2, g2_
    MULFP254
    // stack:           df2, c, g0, f2_, g1, g1_, g2, g2_
    SWAP3
    MULFP254
    // stack:             cf2_, g0, df2, g1, g1_, g2, g2_
    SWAP1 
    SWAP2
    // stack:             df2, cf2_, g0, g1, g1_, g2, g2_
    ADDFP254
    // stack:                   g0_, g0, g1, g1_, g2, g2_
    SWAP1
    // stack:                   g0, g0_, g1, g1_, g2, g2_
%endmacro

/// cost: 1 i9 (9) + 16 dups + 17 swaps + 12 muls + 6 adds/subs = 60
///
/// G0 + G1t + G2t^2 = (a+bi)t^2 * (F0 + F1t + F2t^2) 
///                  = (c+di)F1 + (c+di)F2t + (a+bi)F0t^2
/// where c+di = (a+bi)(9+i) = (9a-b) + (a+9b)i 
///
/// G0 = (c+di)(f1+f1_i) = (cf1 - df1_) + (df1 + cf1_)i
/// G1 = (a+bi)(f2+f2_i) = (cf2 - df2_) + (df2 + cf2_)i
/// G2 = (a+bi)(f0+f0_i) = (af0 - bf0_) + (bf0 + af0_)i

%macro mul_fp2_fp6_sh2
    // stack:             a, b, f0, f0_, f1, f1_, f2, f2_
    DUP4
    DUP3 
    MULFP254
    // stack:       bf0_, a, b, f0, f0_, f1, f1_, f2, f2_
    DUP4
    DUP3
    MULFP254
    // stack:  af0, bf0_, a, b, f0, f0_, f1, f1_, f2, f2_
    SUBFP254
    // stack:         g2, a, b, f0, f0_, f1, f1_, f2, f2_
    SWAP7
    SWAP3
    // stack:         f0, a, b, f2, f0_, f1, f1_, g2, f2_
    DUP3
    MULFP254
    // stack:        bf0, a, b, f2, f0_, f1, f1_, g2, f2_
    SWAP1
    SWAP4
    // stack:        f0_, bf0, b, f2, a, f1, f1_, g2, f2_
    DUP5 
    MULFP254
    // stack:       af0_, bf0, b, f2, a, f1, f1_, g2, f2_
    ADDFP254 
    // stack:             g2_, b, f2, a, f1, f1_, g2, f2_
    SWAP7
    SWAP3
    // stack:             a, b, f2, f2_, f1, f1_, g2, g2_
    %i9
    // stack:             d, c, f2, f2_, f1, f1_, g2, g2_
    DUP4
    DUP2
    MULFP254
    // stack:       df2_, d, c, f2, f2_, f1, f1_, g2, g2_
    DUP4
    DUP4
    MULFP254
    // stack:  cf2, df2_, d, c, f2, f2_, f1, f1_, g2, g2_
    SUBFP254
    // stack:         g1, d, c, f2, f2_, f1, f1_, g2, g2_
    SWAP5
    SWAP3
    // stack:         f2, d, c, f1, f2_, g1, f1_, g2, g2_
    DUP2
    MULFP254
    // stack:        df2, d, c, f1, f2_, g1, f1_, g2, g2_
    SWAP1
    SWAP4
    // stack:        f2_, df2, c, f1, d, g1, f1_, g2, g2_
    DUP3
    MULFP254
    // stack:       cf2_, df2, c, f1, d, g1, f1_, g2, g2_
    ADDFP254
    // stack:             g1_, c, f1, d, g1, f1_, g2, g2_
    SWAP5 
    // stack:             f1_, c, f1, d, g1, g1_, g2, g2_
    DUP1
    DUP5 
    MULFP254
    // stack:       df1_, f1_, c, f1, d, g1, g1_, g2, g2_
    DUP4
    DUP4
    MULFP254
    // stack:  cf1, df1_, f1_, c, f1, d, g1, g1_, g2, g2_
    SUBFP254
    // stack:         g0, f1_, c, f1, d, g1, g1_, g2, g2_
    SWAP3
    // stack:         f1, f1_, c, g0, d, g1, g1_, g2, g2_
    SWAP2
    MULFP254
    // stack:           cf1_, f1, g0, d, g1, g1_, g2, g2_
    SWAP3 
    MULFP254
    // stack:             df1, g0, cf1_, g1, g1_, g2, g2_
    SWAP1
    SWAP2
    // stack:             cf1_, df1, g0, g1, g1_, g2, g2_
    ADDFP254
    // stack:                   g0_, g0, g1, g1_, g2, g2_
    SWAP1
    // stack:                   g0, g0_, g1, g1_, g2, g2_
%endmacro
