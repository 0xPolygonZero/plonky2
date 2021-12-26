Partial rounds ASM
==================

The partial rounds are written in hand-rolled ASM. This was necessary to ensure proper pipelining. Indeed, the ASM shaves 40% off the execution time of the original vector intrinsics-based partial round.

The partial layer performs two operations:
  1. Apply the S-box to state[0]
  2. Apply an affine transform (MDS matrix + constant layer) to the entire state vector.

The S-box must be performed in scalar to minimize latency. The MDS matrix is done mostly in vector to maximize throughput. To take advantage of the otherwise idle scalar execution units, MDS matrix multiplication for result[0..2] is done in scalar. Clearly, this necessitates some data movement, as the input state must be available to both scalar and vector execution units.

This task has plentiful opportunities for pipelining and parallelism. Most immediately, the S-box—with its long latency chain—can be performed simultaneously with most of the MDS matrix multiplication, with the permuted input only available right before the reduction. In addition, the MDS matrix multiplication can be scheduled in a way that interleaves different kinds of operations, masking the latency of the reduction step.

There are three chains of ASM:
  1. the S-box,
  2. the scalar part of MDS multiplication (for result[0..2]),
  3. the vector part of MDS multiplication (for result[2..12]).
Those chains are explained individually below. They interact sporadically to exchange results. In the compiled file, they have been interleaved.


S-box
-----

The ASM for the S-box is as follows:
```assembly
    umulh  {t0}, {s0}, {s0}
    mul    {t1}, {s0}, {s0}
    subs   {t1}, {t1}, {t0}, lsr #32
    csetm  {t2:w}, cc
    lsl    {t3}, {t0}, #32
    sub    {t1}, {t1}, {t2}
    mov    {t0:w}, {t0:w}
    sub    {t0}, {t3}, {t0}
    adds   {t0}, {t1}, {t0}
    csetm  {t1:w}, cs
    add    {t0}, {t0}, {t1}

    // t0 now contains state ** 2
    umulh  {t1}, {s0}, {t0}
    umulh  {t2}, {t0}, {t0}
    mul    {s0}, {s0}, {t0}
    mul    {t0}, {t0}, {t0}
    subs   {s0}, {s0}, {t1}, lsr #32
    csetm  {t3:w}, cc
    subs   {t0}, {t0}, {t2}, lsr #32
    csetm  {t4:w}, cc
    lsl    {t5}, {t1}, #32
    lsl    {t6}, {t2}, #32
    sub    {s0}, {s0}, {t3}
    sub    {t0}, {t0}, {t4}
    mov    {t1:w}, {t1:w}
    mov    {t2:w}, {t2:w}
    sub    {t1}, {t5}, {t1}
    sub    {t2}, {t6}, {t2}
    adds   {t1}, {s0}, {t1}
    csetm  {s0:w}, cs
    adds   {t2}, {t0}, {t2}
    csetm  {t0:w}, cs
    add    {t1}, {t1}, {s0}
    add    {t2}, {t2}, {t0}

    // t1 now contains state ** 3
    // t2 now contains state ** 4
    umulh  {s0}, {t1}, {t2}
    mul    {t0}, {t1}, {t2}
    subs   {t0}, {t0}, {s0}, lsr #32
    csetm  {t1:w}, cc
    lsl    {t2}, {s0}, #32
    sub    {t0}, {t0}, {t1}
    mov    {s0:w}, {s0:w}
    sub    {s0}, {t2}, {s0}
    adds   {s0}, {t0}, {s0}
    csetm  {t0:w}, cs
    add    {s0}, {s0}, {t0}

    // s0 now contains state **7
    fmov   d20, {s0}
```

It is merely four repetitions of a block of 11 instructions (the middle two repetitions are interleaved). The input and output are in `s0`. `t0` through `t6` are scratch registers. The `fmov` copies the result to the bottom 64 bits of the vector register v20.

Trick: `csetm` sets its destination to all 1s if the condition is met. In our case the destination is 32-bits and the condition is overflow/underflow of the previous instruction, so we get EPSILON on over/underflow and 0 otherwise.

Note: the last multiplication does not use `t3` through `t6`, making them available to scalar MDS multiplication.


Scalar MDS multiplication
-------------------------

The ASM for the scalar MDS multiplication is
```assembly
    ldp    {lo0}, {lo1}, [{rc_ptr}]
    add    {lo1}, {lo1}, {s1:w}, uxtw
    add    {lo0}, {lo0}, {s1:w}, uxtw
    lsr    {hi0}, {s1}, #32
    lsr    {t3}, {s2}, #32
    lsr    {t4}, {s3}, #32
    add    {hi1}, {hi0}, {t3}
    add    {hi0}, {hi0}, {t3}, lsl #1
    add    {lo1}, {lo1}, {s2:w}, uxtw
    add    {lo0}, {lo0}, {s2:w}, uxtw #1
    lsr    {t3}, {s4}, #32
    lsr    {t5}, {s5}, #32
    add    {hi1}, {hi1}, {t4}, lsl #1
    add    {t6}, {t3}, {t5}, lsl #3
    add    {t5}, {t3}, {t5}, lsl #2
    lsr    {t3}, {s6}, #32
    lsr    {s1}, {s7}, #32
    mov    {s2:w}, {s4:w}
    add    {hi0}, {hi0}, {t4}
    add    {lo1}, {lo1}, {s3:w}, uxtw #1
    add    {lo0}, {lo0}, {s3:w}, uxtw
    add    {t4}, {s2}, {s5:w}, uxtw #3
    add    {s2}, {s2}, {s5:w}, uxtw #2
    add    {s3}, {s1}, {t3}, lsl #4
    add    {hi1}, {hi1}, {t6}
    add    {hi0}, {hi0}, {t5}, lsl #3
    mov    {t5:w}, {s6:w}
    mov    {t6:w}, {s7:w}
    add    {s4}, {t6}, {t5}, lsl #4
    add    {t3}, {t3}, {s1}, lsl #7
    lsr    {s1}, {s8}, #32
    lsr    {s5}, {s9}, #32
    add    {lo1}, {lo1}, {t4}
    add    {lo0}, {lo0}, {s2}, lsl #3
    add    {t4}, {t5}, {t6}, lsl #7
    add    {hi1}, {hi1}, {s3}, lsl #1
    add    {t5}, {s1}, {s5}, lsl #4
    add    {lo1}, {lo1}, {s4}, lsl #1
    add    {hi0}, {hi0}, {t3}, lsl #1
    mov    {t3:w}, {s9:w}
    mov    {t6:w}, {s8:w}
    add    {s2}, {t6}, {t3}, lsl #4
    add    {s1}, {s5}, {s1}, lsl #9
    lsr    {s3}, {s10}, #32
    lsr    {s4}, {s11}, #32
    add    {lo0}, {lo0}, {t4}, lsl #1
    add    {t3}, {t3}, {t6}, lsl #9
    add    {hi1}, {hi1}, {t5}, lsl #8
    add    {t4}, {s3}, {s4}, lsl #13
    add    {lo1}, {lo1}, {s2}, lsl #8
    add    {hi0}, {hi0}, {s1}, lsl #3
    mov    {t5:w}, {s10:w}
    mov    {t6:w}, {s11:w}
    add    {s1}, {t5}, {t6}, lsl #13
    add    {s2}, {s4}, {s3}, lsl #6
    add    {lo0}, {lo0}, {t3}, lsl #3
    add    {t3}, {t6}, {t5}, lsl #6
    add    {hi1}, {hi1}, {t4}, lsl #3
    add    {lo1}, {lo1}, {s1}, lsl #3
    add    {hi0}, {hi0}, {s2}, lsl #10
    lsr    {t4}, {s0}, #32
    add    {lo0}, {lo0}, {t3}, lsl #10
    add    {hi1}, {hi1}, {t4}, lsl #10
    mov    {t3:w}, {s0:w}
    add    {lo1}, {lo1}, {t3}, lsl #10
    add    {hi0}, {hi0}, {t4}
    add    {lo0}, {lo0}, {t3}

    // Reduction
    lsl    {t0}, {hi0}, #32
    lsl    {t1}, {hi1}, #32
    adds   {lo0}, {lo0}, {t0}
    csetm  {t0:w}, cs
    adds   {lo1}, {lo1}, {t1}
    csetm  {t1:w}, cs
    and    {t2}, {hi0}, #0xffffffff00000000
    and    {t3}, {hi1}, #0xffffffff00000000
    lsr    {hi0}, {hi0}, #32
    lsr    {hi1}, {hi1}, #32
    sub    {hi0}, {t2}, {hi0}
    sub    {hi1}, {t3}, {hi1}
    add    {lo0}, {lo0}, {t0}
    add    {lo1}, {lo1}, {t1}
    adds   {lo0}, {lo0}, {hi0}
    csetm  {t0:w}, cs
    adds   {lo1}, {lo1}, {hi1}
    csetm  {t1:w}, cs
    add    {s0}, {lo0}, {t0}
    add    {s1}, {lo1}, {t1}
```

The MDS multiplication is done separately on the low 32 bits and the high 32 bits of the input, and combined by linearity. Each input is split into the low part and the high part. There are separate accumulators for the low and high parts of the result `lo0`/`lo1`, for result[0] and result[1] respectively, and `hi0`/`hi1`.

The pointer to the round constants is given in `rc_ptr`. Registers `s0`-`s11` contain the state vector at the start, and are later used as scratch. `t3`-`t6` are temporaries.

`s1` is assumed to be available first, as it is computed in scalar. `s2`-`s11` are used next. `s0` is assumed to be available last, as it must be transformed by the S-box.

The reduction is
```assembly
	lsl    {t0}, {hi0}, #32
	adds   {lo0}, {lo0}, {t0}
	csetm  {t0:w}, cs
	and    {t2}, {hi0}, #0xffffffff00000000
	lsr    {hi0}, {hi0}, #32
	sub    {hi0}, {t2}, {hi0}
	add    {lo0}, {lo0}, {t0}
	adds   {lo0}, {lo0}, {hi0}
	csetm  {t0:w}, cs
	add    {s0}, {lo0}, {t0}
```
repeated and interleaved. `cset` sets its destination to EPSILON if the previous instruction overflowed.


Vector MDS multiplication
-------------------------

The ASM for the vector MDS multiplication is
```assembly
	fmov   d21, {s1}

    // res2,3 <- consts,state1
    ldp d0, d1, [{rc_ptr}, #16]
    ushll.2d   v10, v21, #10       // MDS[11] == 10
    ushll.2d   v11, v21, #16       // MDS[10] == 16

    // res2,3 <- state2,3
    uaddw.2d   v0, v0, v22         // MDS[0]  == 0
    umlal.2d   v1, v22, v31[1]     // MDS[11] == 10
    uaddw2.2d  v10, v10, v22       // MDS[1]  == 0
    uaddw2.2d  v11, v11, v22       // MDS[0]  == 0

    // res4,5 <- consts,state1
    ldp d2, d3, [{rc_ptr}, #32]
    ushll.2d   v12, v21, #3        // MDS[9] == 3
    ushll.2d   v13, v21, #12       // MDS[8] == 12

    // res2,3 <- state4,5
    umlal.2d   v0, v23, v30[1]     // MDS[2]  == 1
    uaddw2.2d  v10, v10, v23       // MDS[3]  == 0
    uaddw.2d   v11, v11, v23       // MDS[1]  == 0
    umlal2.2d  v1, v23, v30[1]     // MDS[2]  == 1

    // res4,5 <- state2,3
    umlal.2d   v2, v22, v31[3]     // MDS[10] == 16
    umlal2.2d  v12, v22, v31[1]    // MDS[11] == 10
    umlal.2d   v3, v22, v30[2]     // MDS[9]  == 3
    umlal2.2d  v13, v22, v31[3]    // MDS[10] == 16

    // res6,7 <- consts,state1
    ldp d4, d5, [{rc_ptr}, #48]
    ushll.2d   v14, v21, #8        // MDS[7] == 8
    ushll.2d   v15, v21, #1        // MDS[6] == 1

    // res2,3 <- state6,7
    umlal.2d   v0, v24, v30[2]     // MDS[4]  == 3
    umlal2.2d  v10, v24, v30[3]    // MDS[5]  == 5
    umlal2.2d  v11, v24, v30[2]    // MDS[4]  == 3
    uaddw.2d   v1, v1, v24         // MDS[3]  == 0

    // res4,5 <- state4,5
    uaddw.2d   v2, v2, v23         // MDS[0]  == 0
    umlal.2d   v3, v23, v31[1]     // MDS[11] == 10
    uaddw2.2d  v12, v12, v23       // MDS[1]  == 0
    uaddw2.2d  v13, v13, v23       // MDS[0]  == 0

    // res6,7 <- state2,3
    umlal.2d   v4, v22, v31[2]     // MDS[8]  == 12
    umlal2.2d  v14, v22, v30[2]    // MDS[9]  == 3
    umlal.2d   v5, v22, v31[0]     // MDS[7]  == 8
    umlal2.2d  v15, v22, v31[2]    // MDS[8]  == 12

    // res8,9 <- consts,state1
    ldp d6, d7, [{rc_ptr}, #64]
    ushll.2d   v16, v21, #5        // MDS[5] == 5
    ushll.2d   v17, v21, #3        // MDS[4] == 3

    // res2,3 <- state8,9
    umlal.2d   v0, v25, v30[1]     // MDS[6]  == 1
    umlal2.2d  v10, v25, v31[0]    // MDS[7]  == 8
    umlal.2d   v1, v25, v30[3]     // MDS[5]  == 5
    umlal2.2d  v11, v25, v30[1]    // MDS[6]  == 1

    // res4,5 <- state6,7
    umlal.2d   v2, v24, v30[1]     // MDS[2]  == 1
    uaddw2.2d  v12, v12, v24       // MDS[3]  == 0
    uaddw.2d   v13, v13, v24       // MDS[1]  == 0
    umlal2.2d  v3, v24, v30[1]     // MDS[2]  == 1

    // res6,7 <- state4,5
    umlal.2d   v4, v23, v31[3]     // MDS[10] == 16
    umlal2.2d  v14, v23, v31[1]    // MDS[11] == 10
    umlal.2d   v5, v23, v30[2]     // MDS[9]  == 3
    umlal2.2d  v15, v23, v31[3]    // MDS[10] == 16

    // res8,9 <- state2,3
    umlal.2d   v6, v22, v30[1]     // MDS[6]  == 1
    umlal2.2d  v16, v22, v31[0]    // MDS[7]  == 8
    umlal.2d   v7, v22, v30[3]     // MDS[5]  == 5
    umlal2.2d  v17, v22, v30[1]    // MDS[6]  == 1

    // res10,11 <- consts,state1
    ldp d8, d9, [{rc_ptr}, #80]
    ushll.2d   v18, v21, #0        // MDS[3] == 0
    ushll.2d   v19, v21, #1        // MDS[2] == 1

    // res2,3 <- state10,11
    umlal.2d   v0, v26, v31[2]     // MDS[8]  == 12
    umlal2.2d  v10, v26, v30[2]    // MDS[9]  == 3
    umlal.2d   v1, v26, v31[0]     // MDS[7]  == 8
    umlal2.2d  v11, v26, v31[2]    // MDS[8]  == 12

    // res4,5 <- state8,9
    umlal.2d   v2, v25, v30[2]     // MDS[4]  == 3
    umlal2.2d  v12, v25, v30[3]    // MDS[5]  == 5
    umlal2.2d  v13, v25, v30[2]    // MDS[4]  == 3
    uaddw.2d   v3, v3, v25         // MDS[3]  == 0

    // res6,7 <- state6,7
    uaddw.2d   v4, v4, v24         // MDS[0]  == 0
    umlal.2d   v5, v24, v31[1]     // MDS[11] == 10
    uaddw2.2d  v14, v14, v24       // MDS[1]  == 0
    uaddw2.2d  v15, v15, v24       // MDS[0]  == 0

    // res8,9 <- state4,5
    umlal.2d   v6, v23, v31[2]      // MDS[8]  == 12
    umlal2.2d  v16, v23, v30[2]     // MDS[9]  == 3
    umlal.2d   v7, v23, v31[0]      // MDS[7]  == 8
    umlal2.2d  v17, v23, v31[2]     // MDS[8]  == 12

    // res10,11 <- state2,3
    umlal.2d   v8, v22, v30[2]     // MDS[4]  == 3
    umlal2.2d  v18, v22, v30[3]    // MDS[5]  == 5
    uaddw.2d   v9, v9, v22         // MDS[3]  == 0
    umlal2.2d  v19, v22, v30[2]    // MDS[4]  == 3

    // merge accumulators, res2,3 <- state0, and reduce
    add.2d     v0, v0, v10
    add.2d     v1, v1, v11

    umlal.2d   v0, v20, v31[3]     // MDS[10] == 16
    umlal.2d   v1, v20, v30[2]     // MDS[9]  == 3
    mds_reduce_asm(v0, v1, v22)
    fmov       {s2}, d22
    fmov.d     {s3}, v22[1]

    // res4,5 <- state10,11
    umlal.2d   v2, v26, v30[1]     // MDS[6]  == 1
    umlal2.2d  v12, v26, v31[0]    // MDS[7]  == 8
    umlal.2d   v3, v26, v30[3]     // MDS[5]  == 5
    umlal2.2d  v13, v26, v30[1]    // MDS[6]  == 1

    // res6,7 <- state8,9
    umlal.2d   v4, v25, v30[1]     // MDS[2]  == 1
    uaddw2.2d  v14, v14, v25       // MDS[3]  == 0
    uaddw.2d   v15, v15, v25       // MDS[1]  == 0
    umlal2.2d  v5, v25, v30[1]     // MDS[2]  == 1

    // res8,9 <- state6,7
    umlal.2d   v6, v24, v31[3]     // MDS[10] == 16
    umlal2.2d  v16, v24, v31[1]    // MDS[11] == 10
    umlal.2d   v7, v24, v30[2]     // MDS[9]  == 3
    umlal2.2d  v17, v24, v31[3]    // MDS[10] == 16

    // res10,11 <- state4,5
    umlal.2d   v8, v23, v30[1]     // MDS[6]  == 1
    umlal2.2d  v18, v23, v31[0]    // MDS[7]  == 8
    umlal.2d   v9, v23, v30[3]     // MDS[5]  == 5
    umlal2.2d  v19, v23, v30[1]    // MDS[6]  == 1

    // merge accumulators, res4,5 <- state0, and reduce
    add.2d     v2, v2, v12
    add.2d     v3, v3, v13

    umlal.2d   v2, v20, v31[2]     // MDS[8] == 12
    umlal.2d   v3, v20, v31[0]     // MDS[7]  == 8
    mds_reduce_asm(v2, v3, v23)
    fmov       {s4}, d23
    fmov.d     {s5}, v23[1]

    // res6,7 <- state10,11
    umlal.2d   v4, v26, v30[2]     // MDS[4]  == 3
    umlal2.2d  v14, v26, v30[3]    // MDS[5]  == 5
    umlal2.2d  v15, v26, v30[2]    // MDS[4]  == 3
    uaddw.2d   v5, v5, v26         // MDS[3]  == 0

    // res8,9 <- state8,9
    uaddw.2d   v6, v6, v25         // MDS[0]  == 0
    uaddw2.2d  v16, v16, v25       // MDS[1]  == 0
    uaddw2.2d  v17, v17, v25       // MDS[0]  == 0
    umlal.2d   v7, v25, v31[1]     // MDS[11] == 10

    // res10,11 <- state6,7
    umlal.2d   v8, v24, v31[2]     // MDS[8]  == 12
    umlal2.2d  v18, v24, v30[2]    // MDS[9]  == 3
    umlal.2d   v9, v24, v31[0]     // MDS[7]  == 8
    umlal2.2d  v19, v24, v31[2]    // MDS[8]  == 12

    // merge accumulators, res6,7 <- state0, and reduce
    add.2d     v4, v4, v14
    add.2d     v5, v5, v15

    umlal.2d   v4, v20, v30[1]     // MDS[6]  == 1
    umlal.2d   v5, v20, v30[3]     // MDS[5]  == 5
    mds_reduce_asm(v4, v5, v24)
    fmov       {s6}, d24
    fmov.d     {s7}, v24[1]

    // res8,9 <- state10,11
    umlal.2d   v6, v26, v30[1]     // MDS[2]  == 1
    uaddw2.2d  v16, v16, v26       // MDS[3]  == 0
    umlal2.2d  v17, v26, v30[1]    // MDS[2]  == 1
    uaddw.2d   v7, v7, v26         // MDS[1]  == 0

    // res10,11 <- state8,9
    umlal.2d   v8, v25, v31[3]     // MDS[10] == 16
    umlal2.2d  v18, v25, v31[1]    // MDS[11] == 10
    umlal.2d   v9, v25, v30[2]     // MDS[9]  == 3
    umlal2.2d  v19, v25, v31[3]    // MDS[10] == 16

    // merge accumulators, res8,9 <- state0, and reduce
    add.2d     v6, v6, v16
    add.2d     v7, v7, v17

    umlal.2d   v6, v20, v30[2]     // MDS[4]  == 3
    uaddw.2d   v7, v7, v20         // MDS[3]  == 0
    mds_reduce_asm(v6, v7, v25)
    fmov       {s8}, d25
    fmov.d     {s9}, v25[1]

    // res10,11 <- state10,11
    uaddw.2d   v8, v8, v26         // MDS[0]  == 0
    uaddw2.2d  v18, v18, v26       // MDS[1]  == 0
    umlal.2d   v9, v26, v31[1]     // MDS[11] == 10
    uaddw2.2d  v19, v19, v26       // MDS[0]  == 0

    // merge accumulators, res10,11 <- state0, and reduce
    add.2d     v8, v8, v18
    add.2d     v9, v9, v19

    umlal.2d   v8, v20, v30[1]     // MDS[2]  == 1
    uaddw.2d   v9, v9, v20         // MDS[1]  == 0
    mds_reduce_asm(v8, v9, v26)
    fmov       {s10}, d26
    fmov.d     {s11}, v26[1]
```
where the macro `mds_reduce_asm` is defined as
```assembly
	($c0, $c1, $out) => {
        // Swizzle
        zip1.2d  $out, $c0, $c1  // lo
        zip2.2d  $c0, $c0, $c1   // hi

        // Reduction from u96
        usra.2d  $c0, $out, #32
        sli.2d   $out, $c0, #32
        // Extract high 32-bits.
        uzp2.4s  $c0, $c0, $c0
        // Multiply by EPSILON and accumulate.
        mov.16b  $c1, $out
        umlal.2d $out, $c0, v30[0]
        cmhi.2d  $c1, $c1, $out
        usra.2d  $out, $c1, #32
    }
```

The order in which inputs are assumed to be available is:
- state[1]
- state[2] and state[3]
- state[4] and state[5]
- state[6] and state[7]
- state[8] and state[9]
- state[10] and state[11]
- state[0]

The order in which the results are produced is:
- state[2] and state[3]
- state[4] and state[5]
- state[6] and state[7]
- state[8] and state[9]
- state[10] and state[11]

The order of the instructions in the assembly should be thought of as a setting the relative priority of each instruction; because of CPU reordering, it does not correspond exactly to execution order in time. Ideally, we'd like the MDS matrix multiplication to happen in the following order:
                         s[1]    s[2..4]    s[4..6]    s[6..8]   s[8..10]  s[10..12]       s[0]
         res[2..4]          1          2         4           7         11         16         21
         res[4..6]          3          5         8          12         17         22         26
output   res[6..8]          6          9        13          18         23         27         30
        res[8..10]         10         14        19          24         28         31         33
       res[10..12]         15         20        25          29         32         34         35

This is the order in which the operations are ordered in the ASM. It permits the start of one iteration to be interleaved with the end of the previous iteration (CPU reordering means we don't have to do it manually). Reductions, which have high latency, are executed as soon as the unreduced product is available; the pipelining permits them to be executed simultaneously with multiplication/accumulation, masking the latency.

The registers `v0`-`v19` are used for scratch. `v0` and `v10` are accumulators for res[2], `v1` and `v11` are accumulators for res[3], and so on. The accumulators hold the low result in the low 64 bits and the high result in the high 64 bits (this is convenient as both low and high are always multiplied by the same constant). They must be added before reduction.

The inputs for state[0] and state[1] are in the low 64 bits of `v20` and `v21`, respectively. The inputs and outputs for state[2..4], ..., state[10..12] are in `v22`, ..., `v26`, respectively.

`v30` and `v31` contains the constants [EPSILON, 1 << 1, 1 << 3, 1 << 5], [1 << 8, 1 << 10, 1 << 12, 1 << 16]. EPSILON is used in the reduction. The remaining constants are MDS matrix elements (except 1, which is omitted) and are used to form the dot products.

The instruction `umlal.2d v4, v20, v30[1]` can be read as:
1. take the low 64 bits (`umlal2` for high 64 bits) of `v20` (state[0]),
2. multiply the low and high 32 bits thereof by `v30[1]` (1),
3. add the low and high product to the low and high 64-bits of `v4` respectively,
4. save to `v4`.

We do not use `umlal` when the MDS coefficient is 1; instead, we use `uaddw` ("widening add") to reduce latency.
