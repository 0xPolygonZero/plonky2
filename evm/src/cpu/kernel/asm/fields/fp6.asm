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

// cost: store (40) + i9 (9) = 49
%macro store_fp6_sh(offset)
    // stack: x0, x1, x2, x3, x4, x5
    PUSH $offset
    %add_const(2)
    %mstore_kernel_general
    // stack:     x1, x2, x3, x4, x5
    PUSH $offset
    %add_const(3)
    %mstore_kernel_general
    // stack:         x2, x3, x4, x5
    PUSH $offset
    %add_const(4)
    %mstore_kernel_general
    // stack:             x3, x4, x5
    PUSH $offset
    %add_const(5)
    %mstore_kernel_general
    // stack:                 x4, x5
    %i9
    // stack:                 y5, y4
    PUSH $offset
    %add_const(1)
    %mstore_kernel_general
    // stack:                     y4
    PUSH $offset
    %mstore_kernel_general
    // stack:
%endmacro

// cost: 9; note this returns y, x for x + yi
%macro i9
    // stack:          a , b
    DUP2
    DUP2
    // stack:  a , b,  a , b
    PUSH 9
    MULFP254
    SUBFP254
    // stack: 9a - b,  a , b
    SWAP2 
    // stack:  b , a, 9a - b
    PUSH 9
    MULFP254
    ADDFP254
    // stack: 9b + a, 9a - b 
%endmacro

// cost: 6
%macro dup1_fp6
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
%macro dup2_fp6
    // stack:       f: 6, g: 6
    DUP12
    DUP12
    DUP12
    DUP12
    DUP12
    DUP12
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

// cost: 156
%macro mul_fp6
    /// C = C0 + C1t + C2t^2 
    ///   = (c0 + c0_i) + (c1 + c1_i)t + (c2 + c2_i)t^2
    ///
    /// D = D0 + D1t + D2t^2
    ///   = (d0 + d0_i) + (d1 + d1_i)t + (d2 + d2_i)t^2
    ///
    /// E = E0 + E1t + E2t^2 = CD
    ///
    /// initial stack: c0, c0_, c1, c1_, c2, c2_, d0, d0_, d1, d1_, d2, d2_
    /// final   stack: e0, e0_, e1, e1_, e2, e2_

    /// E0 = C0D0 + i9(C1D2 + C2D1)
    ///
    /// C0D0 = (c0d0 - c0_d0_) + (c0d0_ + c0_d0)i
    ///
    /// C1D2 = (c1d2 - c1_d2_) + (c1d2_ + c1_d2)i
    /// C2D1 = (c2d1 - c2_d1_) + (c2d1_ + c2_d1)i
    ///
    /// CDX  = C1D2 + C2D1
    ///      = (c1d2 + c2d1 - c1_d2_ - c2_d1_) + (c1d2_ + c1_d2 + c2d1_ + c2_d1)i
    ///
    /// i9(CDX) = (9CDX - CDX_) + (CDX + 9CDX_)i
    ///
    /// E0  = 9CDX  - CDX_ + C0D0
    /// E0_ = 9CDX_ + CDX  + C0D0_

    // make CDX_ = c1d2_ + c1_d2 + c2d1_ + c2_d1
    DUP12
    DUP4
    MULFP254
    DUP12
    DUP6
    MULFP254
    ADDFP254
    DUP11
    DUP7
    MULFP254
    ADDFP254
    DUP10
    DUP8
    MULFP254
    ADDFP254
    // make C0D0_ = c0d0_ + c0_d0
    DUP9
    DUP3
    MULFP254
    DUP9
    DUP5
    MULFP254
    ADDFP254
    // make CDX = c1d2 + c2d1 - c1_d2_ - c2_d1_
    DUP12
    DUP9
    MULFP254
    DUP15
    DUP8
    MULFP254
    ADDFP254
    DUP14
    DUP7
    MULFP254
    DUP13
    DUP10
    MULFP254
    ADDFP254
    SUBFP254
    // make C0D0 = c0d0 - c0_d0_
    DUP11
    DUP6
    MULFP254
    DUP11
    DUP6
    MULFP254
    SUBFP254

    // stack:                    C0D0 , CDX , C0D0_, CDX_
    DUP4
    DUP3
    // stack:       CDX , CDX_ , C0D0 , CDX , C0D0_, CDX_
    PUSH 9
    MULFP254
    SUBFP254
    ADDFP254
    // stack: E0 = 9CDX - CDX_ + C0D0 , CDX , C0D0_, CDX_
    SWAP15
    SWAP3
    // stack:                    CDX_ , CDX , C0D0_
    PUSH 9
    MULFP254
    ADDFP254
    ADDFP254
    // stack:             E0_ = 9CDX_ + CDX + C0D0_
    SWAP9
    
    /// E1 = C0D1 + C1D0 + i9(C2D2)
    ///
    /// C0D1 = (c0d1 - c0_d1_) + (c0d1_ + c0_d1)i
    /// C1D0 = (c1d0 - c1_d0_) + (c1d0_ + c1_d0)i
    ///
    /// CD01  = c0d1  + c1d0  - (c0_d1_ + c1_d0_)
    /// CD01_ = c0d1_ + c0_d1 +  c1d0_  + c1_d0
    ///
    ///    C2D2  = (c2d2 - c2_d2_) + (c2d2_ + c2_d2)i
    /// i9(C2D2) = (9C2D2 - C2D2_) + (C2D2 + 9C2D2_)i
    ///
    /// E1  = 9C2D2 -  C2D2_ + CD01
    /// E1_ =  C2D2 + 9C2D2_ + CD01_

    // make C2D2_ = c2d2_ + c2_d2
    DUP13
    DUP9
    MULFP254
    DUP3
    DUP9
    MULFP254
    ADDFP254
    // make C2D2  = c2d2  - c2_d2_
    DUP3
    DUP10
    MULFP254
    DUP15
    DUP10
    MULFP254
    SUBFP254
    // make C0D0 = c0d1 + c1d0 - (c0_d1_ + c1_d0_)
    DUP3
    DUP9
    MULFP254
    DUP15
    DUP8
    MULFP254
    ADDFP254
    DUP12
    DUP9
    MULFP254
    DUP15
    DUP8
    MULFP254
    ADDFP254
    SUBFP254
    // stack:                      C0D0, C2D2, C2D2_
    DUP3
    DUP3
    // stack:       C2D2 , C2D2_ , C0D0, C2D2, C2D2_
    PUSH 9
    MULFP254
    SUBFP254
    ADDFP254
    // stack: E1 = 9C2D2 - C2D2_ + C0D0, C2D2, C2D2_
    SWAP13
    SWAP2
    // stack:                     C2D2_, C2D2
    PUSH 9
    MULFP254
    ADDFP254
    // stack: 9C2D2_ + C2D2
    // make CD01_ = c0d1_ + c0_d1 +  c1d0_  + c1_d0
    DUP11
    DUP9
    MULFP254
    DUP4
    DUP9
    MULFP254
    ADDFP254
    DUP3
    DUP8
    MULFP254
    ADDFP254
    DUP15
    DUP7
    MULFP254
    ADDFP254
    // stack:       CD01_ , 9C2D2_ + C2D2
    ADDFP254
    // stack: E1_ = CD01_ + 9C2D2_ + C2D2
    SWAP13

    /// E2 = C0D2 + C1D1 + C2D0
    ///
    /// C0D2 = (c0d2 - c0_d2_) + (c0d2_ + c0_d2)i
    /// C1D1 = (c1d1 - c1_d1_) + (c1d1_ + c1_d1)i
    /// C2D0 = (c2d0 - c2_d0_) + (c2d0_ + c2_d0)i
    ///
    /// E2  = c0d2  + c1d1  + c2d0  - (c0_d2_ + c1_d1_ + c2_d0_)
    /// E2_ = c0d2_ + c0_d2 + c1d1_ +  c1_d1  + c2d0_  + c2_d0

    // make c0_d2_ + c1_d1_ + c2_d0_
    DUP3
    DUP11
    MULFP254
    DUP2
    DUP10
    MULFP254
    ADDFP254
    DUP5
    DUP8
    MULFP254
    ADDFP254
    // make c0d2  + c1d1  + c2d0
    DUP16
    DUP7
    MULFP254
    DUP4
    DUP10
    MULFP254
    ADDFP254
    DUP13
    DUP12
    MULFP254
    ADDFP254
    // stack:      c0d2  + c1d1  + c2d0 ,  c0_d2_ + c1_d1_ + c2_d0_
    SUBFP254
    // stack: E2 = c0d2  + c1d1  + c2d0 - (c0_d2_ + c1_d1_ + c2_d0_)
    SWAP15
    // make c0d2_ + c0_d2 + c1d1_ +  c1_d1  + c2d0_  + c2_d0
    SWAP7
    MULFP254
    SWAP7
    MULFP254
    SWAP7
    MULFP254
    SWAP2
    MULFP254
    ADDFP254
    SWAP2
    MULFP254
    ADDFP254
    ADDFP254
    ADDFP254
    SWAP2
    MULFP254
    ADDFP254
    // stack: E2_ = c0d2_ + c0_d2 + c1d1_ +  c1_d1  + c2d0_  + c2_d0
    SWAP5
%endmacro
