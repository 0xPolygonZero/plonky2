global mul_Fp6:
    // stack: d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP6
    // stack: c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP12
    // stack: d0_, c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    MULFP254
    // stack: d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP5
    // stack: c0, d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP5
    // stack: d0, c0, d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    MULFP254
    // stack: d0c0, d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    SUBFP254
    // stack: d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP3
    // stack: c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP10
    // stack: d1_, c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    MULFP254
    // stack: d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP14
    // stack: c1_, d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP8
    // stack: d2_, c1_, d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    MULFP254
    // stack: d2_c1_, d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    ADDFP254
    // stack: d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP11
    // stack: c1, d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP10
    // stack: d2, c1, d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    MULFP254
    // stack: d2c1, d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP13
    // stack: c2, d2c1, d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP5
    // stack: d1, c2, d2c1, d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    MULFP254
    // stack: d1c2, d2c1, d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    ADDFP254
    // stack: d1c2 + d2c1, d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    SUBFP254
    // stack: d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP11
    // stack: c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP8
    // stack: d2_, c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    MULFP254
    // stack: d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP15
    // stack: c1_, d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP11
    // stack: d2, c1_, d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    MULFP254
    // stack: d2c1_, d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    ADDFP254
    // stack: d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP13
    // stack: c2, d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP12
    // stack: d1_, c2, d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    MULFP254
    // stack: d1_c2, d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    ADDFP254
    // stack: d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP5
    // stack: c2_, d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP5
    // stack: d1, c2_, d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    MULFP254
    // stack: d1c2_, d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    ADDFP254
    // stack: d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP6
    // stack: d0, d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP10
    // stack: c0_, d0, d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    MULFP254
    // stack: c0_d0, d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP15
    // stack: d0_, c0_d0, d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP9
    // stack: c0, d0_, c0_d0, d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    MULFP254
    // stack: c0d0_, c0_d0, d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    ADDFP254
    // stack: c0d0_ + c0_d0, d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP2
    // stack: d1c2_ + d1_c2 + d2c1_ + d2_c1, c0d0_ + c0_d0, d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    DUP4
    // stack: d1c2 + d2c1 - d2_c1_ + d1_c2_, d1c2_ + d1_c2 + d2c1_ + d2_c1, c0d0_ + c0_d0, d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    PUSH 9
    // stack: 9, d1c2 + d2c1 - d2_c1_ + d1_c2_, d1c2_ + d1_c2 + d2c1_ + d2_c1, c0d0_ + c0_d0, d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    MULFP254
    // stack: 9d1c2 + d2c1 - d2_c1_ + d1_c2_, d1c2_ + d1_c2 + d2c1_ + d2_c1, c0d0_ + c0_d0, d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    SUBFP254
    // stack: 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1, c0d0_ + c0_d0, d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    ADDFP254
    // stack: 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0, d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, c1_
    SWAP15    
    // stack: c1_, d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SWAP3
    // stack: d0c0 - d0_c0_, d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SWAP2
    // stack: d1c2 + d2c1 - d2_c1_ + d1_c2_, d1c2_ + d1_c2 + d2c1_ + d2_c1, d0c0 - d0_c0_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SWAP1
    // stack: d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    PUSH 9
    // stack: 9, d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: 9d1c2_ + d1_c2 + d2c1_ + d2_c1, d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_, d0c0 - d0_c0_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, d1_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SWAP9
    // stack: d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP9
    // stack: d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP5
    // stack: c2_, d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP8
    // stack: d2_, c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP14
    // stack: c2, d2_, c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: c2d2_, c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP8
    // stack: d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP6
    // stack: c2_, d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP11
    // stack: d2, c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP15
    // stack: c2, d2, c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: c2d2, c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SUBFP254
    // stack: c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP15
    // stack: d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP5
    // stack: c1_, d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP4
    // stack: d1_, c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP12
    // stack: c0_, d1_, c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: c0_d1_, c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: c0_d1_ + c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP2
    // stack: c2d2 - c2_d2_, c0_d1_ + c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP4
    // stack: c2d2_ + c2_d2, c2d2 - c2_d2_, c0_d1_ + c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    PUSH 9
    // stack: 9, c2d2_ + c2_d2, c2d2 - c2_d2_, c0_d1_ + c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: 9c2d2_ + c2_d2, c2d2 - c2_d2_, c0_d1_ + c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SUBFP254
    // stack: 9c2d2_ + c2_d2 - c2d2 - c2_d2_, c0_d1_ + c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SUBFP254
    // stack: 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP8
    // stack: d0, 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP15
    // stack: c1, d0, 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: c1d0, 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP7
    // stack: d1, c1d0, 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP11
    // stack: c0, d1, c1d0, 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: c0d1, c1d0, 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: c0d1 + c1d0, 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c1, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SWAP13
    // stack: c1, c2d2 - c2_d2_, c2d2_ + c2_d2, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SWAP2
    // stack: c2d2_ + c2_d2, c2d2 - c2_d2_, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SWAP1
    // stack: c2d2 - c2_d2_, c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    PUSH 9
    // stack: 9, c2d2 - c2_d2_, c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: 9c2d2 - c2_d2_, c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP7
    // stack: d0, 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP5
    // stack: c1_, d0, 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: c1_d0, 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP16
    // stack: d0_, c1_d0, 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP4
    // stack: c1, d0_, c1_d0, 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: c1d0_, c1_d0, 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: c1d0_ + c1_d0, 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP6
    // stack: d1, c1d0_ + c1_d0, 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP12
    // stack: c0_, d1, c1d0_ + c1_d0, 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: c0_d1, c1d0_ + c1_d0, 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: c0_d1 + c1d0_ + c1_d0, 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP4
    // stack: d1_, c0_d1 + c1d0_ + c1_d0, 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP10
    // stack: c0, d1_, c0_d1 + c1d0_ + c1_d0, 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: c0d1_, c0_d1 + c1d0_ + c1_d0, 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: c0d1_ + c0_d1 + c1d0_ + c1_d0, 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SWAP13
    // stack: c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP15
    // stack: d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP7
    // stack: c2_, d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP4
    // stack: d1_, c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP6
    // stack: c1_, d1_, c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: c1_d1_, c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: c1_d1_ + c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP10
    // stack: d2_, c1_d1_ + c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP12
    // stack: c0_, d2_, c1_d1_ + c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: c0_d2_, c1_d1_ + c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: c0_d2_ + c1_d1_ + c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP8
    // stack: d0, c0_d2_ + c1_d1_ + c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP3
    // stack: c2, d0, c0_d2_ + c1_d1_ + c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: c2d0, c0_d2_ + c1_d1_ + c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP7
    // stack: d1, c2d0, c0_d2_ + c1_d1_ + c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP5
    // stack: c1, d1, c2d0, c0_d2_ + c1_d1_ + c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: c1d1, c2d0, c0_d2_ + c1_d1_ + c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: c1d1 + c2d0, c0_d2_ + c1_d1_ + c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP13
    // stack: d2, c1d1 + c2d0, c0_d2_ + c1_d1_ + c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    DUP11
    // stack: c0, d2, c1d1 + c2d0, c0_d2_ + c1_d1_ + c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: c0d2, c1d1 + c2d0, c0_d2_ + c1_d1_ + c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: c0d2 + c1d1 + c2d0, c0_d2_ + c1_d1_ + c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SUBFP254
    // stack: c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SWAP15
    // stack: d0_, c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: d0_c2, c1, d1_, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SWAP2
    // stack: d1_, c1, d0_c2, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: d1_c1, d0_c2, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: d1_c1 + d0_c2, c1_, d1, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SWAP2
    // stack: d1, c1_, d1_c1 + d0_c2, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: d1c1_, d1_c1 + d0_c2, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: d1c1_ + d1_c1 + d0_c2, c2_, d0, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SWAP2
    // stack: d0, c2_, d1c1_ + d1_c1 + d0_c2, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: d0c2_, d1c1_ + d1_c1 + d0_c2, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: d0c2_ + d1c1_ + d1_c1 + d0_c2, c0, d2_, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SWAP2
    // stack: d2_, c0, d0c2_ + d1c1_ + d1_c1 + d0_c2, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: d2_c0, d0c2_ + d1c1_ + d1_c1 + d0_c2, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: d2_c0 + d0c2_ + d1c1_ + d1_c1 + d0_c2, c0_, d2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SWAP2
    // stack: d2, c0_, d2_c0 + d0c2_ + d1c1_ + d1_c1 + d0_c2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    MULFP254
    // stack: d2c0_, d2_c0 + d0c2_ + d1c1_ + d1_c1 + d0_c2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    ADDFP254
    // stack: d2c0_ + d2_c0 + d0c2_ + d1c1_ + d1_c1 + d0_c2, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0
    SWAP5
    // stack: 9d1c2 + d2c1 - d2_c1_ + d1_c2_ - d1c2_ + d1_c2 + d2c1_ + d2_c1 + c0d0_ + c0_d0, 9d1c2_ + d1_c2 + d2c1_ + d2_c1 + d1c2 + d2c1 - d2_c1_ + d1_c2_ + d0c0 - d0_c0_, c0d1 + c1d0 + 9c2d2_ + c2_d2 - c2d2 - c2_d2_ - c0_d1_ + c1_d0_, c0d1_ + c0_d1 + c1d0_ + c1_d0 + 9c2d2 - c2_d2_ + c2d2_ + c2_d2, c0d2 + c1d1 + c2d0 - c0_d2_ + c1_d1_ + c2_d0_, d2c0_ + d2_c0 + d0c2_ + d1c1_ + d1_c1 + d0_c2
    %jump(0xdeadbeef)
