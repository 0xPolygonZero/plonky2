// cost: 156
%macro mul_fp6
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

    // CDX_ = c1d2_ + c1_d2 + c2d1_ + c2_d1
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
    // C0D0_ = c0d0_ + c0_d0
    DUP9
    DUP3
    MULFP254
    DUP9
    DUP5
    MULFP254
    ADDFP254
    // CDX = c1d2 + c2d1 - c1_d2_ - c2_d1_
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
    // C0D0 = c0d0 - c0_d0_
    DUP11
    DUP6
    MULFP254
    DUP11
    DUP6
    MULFP254
    SUBFP254
    // stack:               C0D0 , CDX , C0D0_, CDX_
    DUP4
    DUP3
    // stack:  CDX , CDX_ , C0D0 , CDX , C0D0_, CDX_
    PUSH 9
    MULFP254
    SUBFP254
    ADDFP254
    // stack: 9CDX - CDX_ + C0D0 , CDX , C0D0_, CDX_
    SWAP15
    SWAP3
    // stack:           CDX_ , CDX , C0D0_
    PUSH 9
    MULFP254
    ADDFP254
    ADDFP254
    // stack:           9CDX_ + CDX + C0D0_
    SWAP9
    
    /// E1 = C0D1 + C1D0 + i9(C2D2)
    ///
    /// C0D1 = (c0d1 - c0_d1_) + (c0d1_ + c0_d1)i
    /// C1D0 = (c1d0 - c1_d0_) + (c1d0_ + c1_d0)i
    ///
    ///    C2D2  = (c2d2 - c2_d2_) + (c2d2_ + c2_d2)i
    /// i9(C2D2) = (9C2D2 - C2D2_) + (C2D2 + 9C2D2_)i
    ///
    /// E1  = 9C2D2 -  C2D2_ + c0d1  + c1d0  - (c0_d1_ + c1_d0_)
    /// E1_ =  C2D2 + 9C2D2_ + c0d1_ + c0_d1 +  c1d0_  + c1_d0

    // C2D2_ = c2d2_ + c2_d2
    DUP13
    DUP9
    MULFP254
    DUP3
    DUP9
    MULFP254
    ADDFP254
    // C2D2  = c2d2  - c2_d2_
    DUP3
    DUP10
    MULFP254
    DUP15
    DUP10
    MULFP254
    SUBFP254
    // stack:                                                   C2D2, C2D2_
    // c0d1 + c1d0 - (c0_d1_ + c1_d0_)
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
    // stack:                 c0d1  + c1d0 - (c0_d1_ + c1_d0_), C2D2, C2D2_
    DUP3
    DUP3
    // stack:  C2D2 , C2D2_ , c0d1  + c1d0 - (c0_d1_ + c1_d0_), C2D2, C2D2_
    PUSH 9
    MULFP254
    SUBFP254
    ADDFP254
    // stack: 9C2D2 - C2D2_ + c0d1  + c1d0 - (c0_d1_ + c1_d0_), C2D2, C2D2_
    SWAP13
    SWAP2
    // stack:                                           C2D2_ , C2D2
    PUSH 9
    MULFP254
    ADDFP254
    // stack: 9C2D2_ + C2D2
    // c0d1_ + c0_d1 +  c1d0_  + c1_d0
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
    ADDFP254
    SWAP13
    /// E2 = C0D2 + C1D1 + C2D0
    ///
    /// C0D2 = (c0d2 - c0_d2_) + (c0d2_ + c0_d2)i
    /// C1D1 = (c1d1 - c1_d1_) + (c1d1_ + c1_d1)i
    /// C2D0 = (c2d0 - c2_d0_) + (c2d0_ + c2_d0)i
    ///
    /// E2  = c0d2  + c1d1  + c2d0  - (c0_d2_ + c1_d1_ + c2_d0_)
    /// E2_ = c0d2_ + c0_d2 + c1d1_ +  c1_d1  + c2d0_  + c2_d0
    // c0_d2_ + c1_d1_ + c2_d0_
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
    // c0d2  + c1d1  + c2d0
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
    // stack: c0d2  + c1d1  + c2d0, c0_d2_ + c1_d1_ + c2_d0_
    SUBFP254
    SWAP15
    // c0d2_ + c0_d2 + c1d1_ +  c1_d1  + c2d0_  + c2_d0
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
    SWAP5
%endmacro
