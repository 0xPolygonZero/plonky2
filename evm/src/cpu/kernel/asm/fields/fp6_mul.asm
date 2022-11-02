/// inputs:
///     C = C0 + C1t + C2t^2 
///       = (c0 + c0_i) + (c1 + c1_i)t + (c2 + c2_i)t^2
///
///     D = D0 + D1t + D2t^2
///       = (d0 + d0_i) + (d1 + d1_i)t + (d2 + d2_i)t^2
///
/// output:
///     E = E0 + E1t + E2t^2 = CD
///       = (e0 + e0_i) + (e1 + e1_i)t + (e2 + e2_i)t^2
///
/// initial stack: c0, c0_, c1, c1_, c2, c2_, d0, d0_, d1, d1_, d2, d2_, retdest
/// final   stack: e0, e0_, e1, e1_, e2, e2_

/// computations:
///
/// E0 = C0D0 + i9(C1D2 + C2D1)
///
/// C0D0 = (c0d0 - c0_d0_) + (c0d0_ + c0_d0)i
///
/// C1D2 = (c1d2 - c1_d2_) + (c1d2_ + c1_d2)i
/// C2D1 = (c2d1 - c2_d1_) + (c2d1_ + c2_d1)i
///
/// CD12 = C1D2 + C2D1
///      = (c1d2 + c2d1 - c1_d2_ - c2_d1_) + (c1d2_ + c1_d2 + c2d1_ + c2_d1)i
///
/// i9(CD12) = (9CD12 - CD12_) + (CD12 + 9CD12_)i
///
/// e0  = 9CD12  - CD12_ + C0D0
/// e0_ = 9CD12_ + CD12  + C0D0_
///
///
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
/// e1  = 9C2D2 -  C2D2_ + CD01
/// e1_ =  C2D2 + 9C2D2_ + CD01_
///
///
/// E2 = C0D2 + C1D1 + C2D0
///
/// C0D2 = (c0d2 - c0_d2_) + (c0d2_ + c0_d2)i
/// C1D1 = (c1d1 - c1_d1_) + (c1d1_ + c1_d1)i
/// C2D0 = (c2d0 - c2_d0_) + (c2d0_ + c2_d0)i
///
/// e2  = c0d2  + c1d1  + c2d0  - (c0_d2_ + c1_d1_ + c2_d0_)
/// e2_ = c0d2_ + c0_d2 + c1d1_ +  c1_d1  + c2d0_  + c2_d0


// cost: 157
global mul_fp6:
    // e2
    // make c0_d2_ + c1_d1_ + c2_d0_
    DUP8
    DUP7
    MULFP254
    DUP11
    DUP6
    MULFP254
    ADDFP254
    DUP13
    DUP4
    MULFP254
    ADDFP254
    // make c0d2 + c1d1 + c2d0
    DUP12
    DUP3
    MULFP254
    DUP11
    DUP6
    MULFP254
    ADDFP254
    DUP9
    DUP8
    MULFP254
    ADDFP254
    // stack:      c0d2  + c1d1  + c2d0 ,  c0_d2_ + c1_d1_ + c2_d0_
    SUBFP254
    // stack: e2 = c0d2  + c1d1  + c2d0 - (c0_d2_ + c1_d1_ + c2_d0_)
    SWAP12

    // e0, e0_
    // make CD12_ = c1d2_ + c1_d2 + c2d1_ + c2_d1
    DUP1
    DUP5
    MULFP254
    DUP13
    DUP7
    MULFP254
    ADDFP254
    DUP12
    DUP8
    MULFP254
    ADDFP254
    DUP11
    DUP9
    MULFP254
    ADDFP254
    // make C0D0_ = c0d0_ + c0_d0
    DUP10
    DUP4
    MULFP254
    DUP10
    DUP6
    MULFP254
    ADDFP254
    // make CD12 = c1d2 + c2d1 - c1_d2_ - c2_d1_
    DUP13
    DUP10
    MULFP254
    DUP4
    DUP9
    MULFP254
    ADDFP254
    DUP15
    DUP8
    MULFP254
    DUP14
    DUP11
    MULFP254
    ADDFP254
    SUBFP254
    // make C0D0 = c0d0 - c0_d0_
    DUP12
    DUP7
    MULFP254
    DUP12
    DUP7
    MULFP254
    SUBFP254
    // stack:                      C0D0 , CD12 , C0D0_, CD12_
    DUP4
    DUP3
    // stack:       CD12 , CD12_ , C0D0 , CD12 , C0D0_, CD12_
    PUSH 9
    MULFP254
    SUBFP254
    ADDFP254
    // stack: e0 = 9CD12 - CD12_ + C0D0 , CD12 , C0D0_, CD12_
    SWAP12
    SWAP3
    // stack:                     CD12_ , CD12 , C0D0_
    PUSH 9
    MULFP254
    ADDFP254
    ADDFP254
    // stack:              e0_ = 9CD12_ + CD12 + C0D0_
    SWAP11

    // e1, e1_
    // make C2D2_ = c2d2_ + c2_d2
    DUP14
    DUP10
    MULFP254
    DUP4
    DUP10
    MULFP254
    ADDFP254
    // make C2D2  = c2d2  - c2_d2_
    DUP4
    DUP11
    MULFP254
    DUP16
    DUP11
    MULFP254
    SUBFP254
    // make CD01 = c0d1 + c1d0 - (c0_d1_ + c1_d0_)
    DUP4
    DUP10
    MULFP254
    DUP16
    DUP9
    MULFP254
    ADDFP254
    DUP13
    DUP10
    MULFP254
    DUP5
    DUP9
    MULFP254
    ADDFP254
    SUBFP254
    // stack:                      CD01, C2D2, C2D2_
    DUP3
    DUP3
    // stack:       C2D2 , C2D2_ , CD01, C2D2, C2D2_
    PUSH 9
    MULFP254
    SUBFP254
    ADDFP254
    // stack: e1 = 9C2D2 - C2D2_ + CD01, C2D2, C2D2_
    SWAP15
    SWAP2
    // stack:                    C2D2_ , C2D2
    PUSH 9
    MULFP254
    ADDFP254
    // stack:                   9C2D2_ + C2D2
    // make CD01_ = c0d1_ + c0_d1 +  c1d0_  + c1_d0
    DUP12
    DUP10
    MULFP254
    DUP5
    DUP10
    MULFP254
    ADDFP254
    DUP4
    DUP9
    MULFP254
    ADDFP254
    DUP3
    DUP8
    MULFP254
    ADDFP254
    // stack:       CD01_ , 9C2D2_ + C2D2
    ADDFP254
    // stack: e1_ = CD01_ + 9C2D2_ + C2D2
    SWAP15

    // e2_
    // stack: d2, d1_, d1, d0_, d2_, c0, c0_, c1, c1_, c2, c2_, d0
    SWAP7
    MULFP254
    // stack:   c1d1_, d1, d0_, d2_, c0, c0_, d2, c1_, c2, c2_, d0
    SWAP7
    MULFP254
    // stack:     c1_d1, d0_, d2_, c0, c0_, d2, c1d1_, c2, c2_, d0
    SWAP7
    MULFP254
    // stack:      c2d0_, d2_, c0, c0_, d2, c1d1_, c1_d1 , c2_, d0
    SWAP2
    MULFP254
    // stack:       c0d2_ , c2d0_, c0_, d2, c1d1_, c1_d1 , c2_, d0
    ADDFP254
    // stack:       c0d2_ + c2d0_, c0_, d2, c1d1_, c1_d1 , c2_, d0
    SWAP2
    MULFP254
    // stack:      c0_d2 , c0d2_ + c2d0_ , c1d1_ , c1_d1 , c2_, d0
    ADDFP254
    ADDFP254
    ADDFP254
    // stack:      c0_d2 + c0d2_ + c2d0_ + c1d1_ + c1_d1 , c2_, d0
    SWAP2
    MULFP254
    ADDFP254
    // stack:  e2_ = c2_d0 + c0_d2 + c0d2_ + c2d0_ + c1d1_ + c1_d1
    SWAP6

    // stack: retdest, e0, e0_, e1, e1_, e2, e2_
    JUMP
