/// Note: uncomment this to test

global test_mul_fp12:
    // stack: inA, f, f', inB, g, g', mul_dest, inA, inB, out, return_fp12_on_stack, out
    %store_fp12
    // stack:             inB, g, g', mul_dest, inA, inB, out, return_fp12_on_stack, out
    %store_fp12
    // stack:                         mul_dest, inA, inB, out, return_fp12_on_stack, out
    JUMP

///////////////////////////////////////
///// GENERAL FP12 MULTIPLICATION /////
///////////////////////////////////////

/// cost: 1063

/// fp6 functions:
///  fn    | num | ops | cost
///  -------------------------
///  load  |   8 |  40 |  320
///  store |   5 |  40 |  200
///  dup   |   5 |   6 |   30
///  swap  |   4 |  16 |   64
///  add   |   4 |  16 |   64
///  subr  |   1 |  17 |   17
///  mul   |   3 | 157 |  471
///  i9    |   1 |   9 |    9
///
/// lone stack operations:
///  op    | num 
///  ------------
///  ADD   |   3
///  SWAP  |   2
///  DUP   |   6
///  PUSH  |   6
///  POP   |   2
///  JUMP  |   6
///
/// TOTAL: 1201

/// inputs:
///     F = f + f'z
///     G = g + g'z
///
/// output:
///     H = h + h'z = FG
///
///     h  = fg + sh(f'g')
///     h' = (f+f')(g+g') - fg - f'g'
///
/// memory pointers [ind' = ind+6]
///     {inA: f, inA: f', inB: g, inB':g', out: h, out': h'}
///
/// f, f', g, g' consist of six elements on the stack

global mul_fp12:
    // stack:                                inA, inB, out 
    DUP1  %offset_fp6 
    // stack:                          inA', inA, inB, out 
    %load_fp6
    // stack:                            f', inA, inB, out 
    DUP8  %offset_fp6
    // stack:                      inB', f', inA, inB, out 
    %load_fp6
    // stack:                        g', f', inA, inB, out 
    PUSH mul_fp12_1
    // stack:            mul_fp12_1, g', f', inA, inB, out 
    %dup_fp6_7
    // stack:        f', mul_fp12_1, g', f', inA, inB, out 
    %dup_fp6_7
    // stack:    g', f', mul_fp12_1, g', f', inA, inB, out 
    %jump(mul_fp6)
mul_fp12_1:
    // stack:                f'g', g'  , f', inA, inB, out 
    %dup_fp6_0
    // stack:          f'g', f'g', g'  , f', inA, inB, out 
    %store_fp6_sh(0)                                    
    // stack:                f'g', g'  , f', inA, inB, out  {0: sh(f'g')}
    %store_fp6(6)
    // stack:                      g'  , f', inA, inB, out  {0: sh(f'g'), 6: f'g'}
    DUP13
    // stack:                 inA, g'  , f', inA, inB, out  {0: sh(f'g'), 6: f'g'}
    DUP15  
    // stack:            inB, inA, g'  , f', inA, inB, out  {0: sh(f'g'), 6: f'g'}
    %load_fp6
    // stack:             g , inA, g'  , f', inA, inB, out  {0: sh(f'g'), 6: f'g'}
    %swap_fp6_hole
    // stack:             g', inA, g   , f', inA, inB, out  {0: sh(f'g'), 6: f'g'}
    %dup_fp6_7
    // stack:           g,g', inA, g   , f', inA, inB, out  {0: sh(f'g'), 6: f'g'}
    %add_fp6
    // stack:           g+g', inA, g   , f', inA, inB, out  {0: sh(f'g'), 6: f'g'}
    %swap_fp6_hole
    // stack:              g, inA, g+g', f', inA, inB, out  {0: sh(f'g'), 6: f'g'}
    PUSH mul_fp12_2
    // stack:  mul_fp12_2, g, inA, g+g', f', inA, inB, out  {0: sh(f'g'), 6: f'g'}
    SWAP7
    // stack:  inA, g, mul_fp12_2, g+g', f', inA, inB, out  {0: sh(f'g'), 6: f'g'}
    %load_fp6
    // stack:    f, g, mul_fp12_2, g+g', f', inA, inB, out  {0: sh(f'g'), 6: f'g'}
    %jump(mul_fp6)
mul_fp12_2:    
    // stack:                  fg, g+g', f', inA, inB, out  {0: sh(f'g'), 6: f'g'}
    %store_fp6(12)
    // stack:                      g+g', f', inA, inB, out  {0: sh(f'g'), 6: f'g', 12: fg}
    %swap_fp6
    // stack:                      f', g+g', inA, inB, out  {0: sh(f'g'), 6: f'g', 12: fg}
    PUSH mul_fp12_3
    // stack:          mul_fp12_3, f', g+g', inA, inB, out  {0: sh(f'g'), 6: f'g', 12: fg}
    SWAP13
    // stack:          inA, f', g+g', mul_fp12_3, inB, out  {0: sh(f'g'), 6: f'g', 12: fg}
    %load_fp6
    // stack:             f,f', g+g', mul_fp12_3, inB, out  {0: sh(f'g'), 6: f'g', 12: fg}
    %add_fp6
    // stack:             f+f', g+g', mul_fp12_3, inB, out  {0: sh(f'g'), 6: f'g', 12: fg}
    %jump(mul_fp6)
mul_fp12_3:
    // stack:                       (f+f')(g+g'), inB, out  {0: sh(f'g'), 6: f'g', 12: fg}
    %load_fp6(12)
    // stack:                   fg, (f+f')(g+g'), inB, out  {0: sh(f'g'), 6: f'g', 12: fg}
    %swap_fp6
    // stack:                   (f+f')(g+g'), fg, inB, out  {0: sh(f'g'), 6: f'g', 12: fg}
    %dup_fp6_6
    // stack:               fg, (f+f')(g+g'), fg, inB, out  {0: sh(f'g'), 6: f'g', 12: fg}
    %load_fp6(6)
    // stack:          f'g',fg, (f+f')(g+g'), fg, inB, out  {0: sh(f'g'), 6: f'g', 12: fg}
    %add_fp6
    // stack:          f'g'+fg, (f+f')(g+g'), fg, inB, out  {0: sh(f'g'), 6: f'g', 12: fg}
    %subr_fp6
    // stack:       (f+f')(g+g') - (f'g'+fg), fg, inB, out  {0: sh(f'g'), 6: f'g', 12: fg}   
    DUP14  %offset_fp6 
    // stack: out', (f+f')(g+g') - (f'g'+fg), fg, inB, out  {0: sh(f'g'), 6: f'g', 12: fg}   
    %store_fp6
    // stack:                                 fg, inB, out  {0: sh(f'g'), 6: f'g', 12: fg}
    %load_fp6(0)
    // stack:                      sh(f'g') , fg, inB, out  {0: sh(f'g'), 6: f'g', 12: fg}
    %add_fp6
    // stack:                      sh(f'g') + fg, inB, out  {0: sh(f'g'), 6: f'g', 12: fg}
    DUP8
    // stack:                 out, sh(f'g') + fg, inB, out  {0: sh(f'g'), 6: f'g', 12: fg}
    %store_fp6
    // stack:                                     inB, out  {0: sh(f'g'), 6: f'g', 12: fg}
    %pop2  JUMP


//////////////////////////////////////
///// SPARSE FP12 MULTIPLICATION /////
//////////////////////////////////////

/// cost: 645

/// fp6 functions:
///  fn      | num | ops | cost
///  ---------------------------
///  load    |   2 |  40 |   80
///  store   |   2 |  40 |   80
///  dup     |   4 |   6 |   24
///  swap    |   4 |  16 |   64
///  add     |   4 |  16 |   64
///  mul_fp  |   2 |  21 |   42
///  mul_fp2 |   4 |  59 |  236
///
/// lone stack operations:
///  op    | num 
///  ------------
///  ADD   |   6
///  DUP   |   9
///  PUSH  |   6
///  POP   |   5
///
/// TOTAL: 618

/// input:
///     F = f + f'z
///     G = g0 + (G1)t + (G2)tz
///
/// output:
///     H = h + h'z = FG
///       = g0 * [f + f'z] + G1 * [sh(f) + sh(f')z] + G2 * [sh2(f') + sh(f)z]
///     
///     h  = g0 * f  + G1 * sh(f ) + G2 * sh2(f') 
///     h' = g0 * f' + G1 * sh(f') + G2 * sh (f )
///
/// memory pointers [ind' = ind+6, inB2 = inB1 + 2 = inB + 3]
///     { inA: f, inA': f', inB: g0, inB1: G1, inB2: G2, out: h, out': h'}
///
/// f, f' consist of six elements; G1, G1' consist of two elements; and g0 of one element 

global mul_fp12_sparse:
    // stack:                                                                    inA, inB, out
    DUP1  %offset_fp6
    // stack:                                                              inA', inA, inB, out
    %load_fp6
    // stack:                                                                f', inA, inB, out
    DUP8 
    // stack:                                                           inB, f', inA, inB, out
    DUP8
    // stack:                                                      inA, inB, f', inA, inB, out
    %load_fp6
    // stack:                                                        f, inB, f', inA, inB, out
    DUP16
    // stack:                                                   out, f, inB, f', inA, inB, out
    %dup_fp6_8 
    // stack:                                               f', out, f, inB, f', inA, inB, out
    DUP14
    // stack:                                          inB, f', out, f, inB, f', inA, inB, out
    %dup_fp6_8
    // stack:                                       f, inB, f', out, f, inB, f', inA, inB, out
    DUP7
    // stack:                                  inB, f, inB, f', out, f, inB, f', inA, inB, out
    %dup_fp6_8
    // stack:                              f', inB, f, inB, f', out, f, inB, f', inA, inB, out
    %dup_fp6_7
    // stack:                           f, f', inB, f, inB, f', out, f, inB, f', inA, inB, out
    DUP13 
    // stack:                      inB, f, f', inB, f, inB, f', out, f, inB, f', inA, inB, out
    %mload_kernel_general
    // stack:                      g0 , f, f', inB, f, inB, f', out, f, inB, f', inA, inB, out
    %mul_fp_fp6
    // stack:                      g0 * f, f', inB, f, inB, f', out, f, inB, f', inA, inB, out
    %swap_fp6
    // stack:                    f'  , g0 * f, inB, f, inB, f', out, f, inB, f', inA, inB, out
    DUP13  %add_const(8)
    // stack:           inB2,    f'  , g0 * f, inB, f, inB, f', out, f, inB, f', inA, inB, out
    %load_fp2
    // stack:           G2  ,    f'  , g0 * f, inB, f, inB, f', out, f, inB, f', inA, inB, out
    %mul_fp2_fp6_sh2
    // stack:           G2 * sh2(f') , g0 * f, inB, f, inB, f', out, f, inB, f', inA, inB, out
    %add_fp6
    // stack:           G2 * sh2(f') + g0 * f, inB, f, inB, f', out, f, inB, f', inA, inB, out
    %swap_fp6_hole
    // stack:          f , inB, G2 * sh2(f') + g0 * f, inB, f', out, f, inB, f', inA, inB, out
    DUP7  %add_const(2)
    // stack: inB1,    f , inB, G2 * sh2(f') + g0 * f, inB, f', out, f, inB, f', inA, inB, out
    %load_fp2
    // stack:  G1 ,    f , inB, G2 * sh2(f') + g0 * f, inB, f', out, f, inB, f', inA, inB, out
    %mul_fp2_fp6_sh
    // stack:  G1 * sh(f), inB, G2 * sh2(f') + g0 * f, inB, f', out, f, inB, f', inA, inB, out
    %add_fp6_hole
    // stack:      G1 * sh(f) + G2 * sh2(f') + g0 * f, inB, f', out, f, inB, f', inA, inB, out
    DUP14
    // stack: out, G1 * sh(f) + G2 * sh2(f') + g0 * f, inB, f', out, f, inB, f', inA, inB, out
    %store_fp6
    // stack:                                          inB, f', out, f, inB, f', inA, inB, out
    %mload_kernel_general
    // stack:                                          g0 , f', out, f, inB, f', inA, inB, out
    %mul_fp_fp6
    // stack:                                          g0 * f', out, f, inB, f', inA, inB, out
    %swap_fp6_hole
    // stack:                                        f  , out, g0 * f', inB, f', inA, inB, out
    DUP14  %add_const(8)
    // stack:                               inB2,    f  , out, g0 * f', inB, f', inA, inB, out
    %load_fp2
    // stack:                                G2 ,    f  , out, g0 * f', inB, f', inA, inB, out
    %mul_fp2_fp6_sh
    // stack:                                G2 * sh(f) , out, g0 * f', inB, f', inA, inB, out
    %add_fp6_hole
    // stack:                                     G2 * sh(f) + g0 * f', inB, f', inA, inB, out
    %swap_fp6_hole
    // stack:                                    f' , inB, G2 * sh(f) + g0 * f', inA, inB, out
    DUP7  %add_const(2)
    // stack:                           inB1,    f' , inB, G2 * sh(f) + g0 * f', inA, inB, out
    %load_fp2
    // stack:                            G1 ,    f' , inB, G2 * sh(f) + g0 * f', inA, inB, out
    %mul_fp2_fp6_sh
    // stack:                            G1 * sh(f'), inB, G2 * sh(f) + g0 * f', inA, inB, out
    %add_fp6_hole
    // stack:                                G1 * sh(f') + G2 * sh(f) + g0 * f', inA, inB, out
    DUP9  %offset_fp6
    // stack:                          out', G1 * sh(f') + G2 * sh(f) + g0 * f', inA, inB, out
    %store_fp6
    // stack:                                                                    inA, inB, out
    %pop3  JUMP


/////////////////////////
///// FP12 SQUARING /////
/////////////////////////

/// cost: 646

/// fp6 functions:
///  fn    | num | ops | cost
///  -------------------------
///  load  |   2 |  40 |   80
///  store |   2 |  40 |   80
///  dup   |   2 |   6 |   12
///  swap  |   2 |  16 |   32
///  add   |   1 |  16 |   16
///  mul   |   1 | 157 |  157
///  sq    |   2 | 101 |  202
///  dbl   |   1 |  13 |   13
///
/// lone stack operations:
///  op    | num 
///  ------------
///  ADD   |   3
///  SWAP  |   4
///  DUP   |   5
///  PUSH  |   6
///  POP   |   3
///  JUMP  |   4
///
/// TOTAL: 

/// input:
///     F = f + f'z
///
/// output:
///     H = h + h'z = FF
///
///     h  = ff + sh(f'f')
///     h' = 2ff'
///
/// memory pointers [ind' = ind+6]
///     {inp: f, inp: f', out: h, out': h'}
///
/// f, f' consist of six elements on the stack

global square_fp12_test:
    POP
    %jump(square_fp12)

global square_fp12:
    // stack:                                                                   inp, out
    DUP1
    // stack:                                                              inp, inp, out
    %load_fp6 
    // stack:                                                                f, inp, out
    PUSH square_fp12_3
    // stack:                                                 square_fp12_3, f, inp, out
    SWAP7
    // stack:                                                 inp, f, square_fp12_3, out
    PUSH square_fp12_2
    // stack:                                  square_fp12_2, inp, f, square_fp12_3, out 
    %dup_fp6_2
    // stack:                              f , square_fp12_2, inp, f, square_fp12_3, out
    DUP16  %offset_fp6
    // stack:                        out', f , square_fp12_2, inp, f, square_fp12_3, out
    PUSH square_fp12_1
    // stack:         square_fp12_1, out', f , square_fp12_2, inp, f, square_fp12_3, out
    DUP10  %offset_fp6
    // stack:   inp', square_fp12_1, out', f , square_fp12_2, inp, f, square_fp12_3, out
    %load_fp6
    // stack:     f', square_fp12_1, out', f , square_fp12_2, inp, f, square_fp12_3, out
    %swap_fp6_hole_2
    // stack:     f , square_fp12_1, out', f', square_fp12_2, inp, f, square_fp12_3, out
    %dup_fp6_8
    // stack: f', f , square_fp12_1, out', f', square_fp12_2, inp, f, square_fp12_3, out
    %jump(mul_fp6)
square_fp12_1:
    // stack:                   f'f, out', f', square_fp12_2, inp, f, square_fp12_3, out
    DUP7
    // stack:             out', f'f, out', f', square_fp12_2, inp, f, square_fp12_3, out
    %store_fp6_double
    // stack:                        out', f', square_fp12_2, inp, f, square_fp12_3, out
    POP
    // stack:                              f', square_fp12_2, inp, f, square_fp12_3, out
    %jump(square_fp6)
square_fp12_2:
    // stack:                                           f'f', inp, f, square_fp12_3, out
    %sh
    // stack:                                       sh(f'f'), inp, f, square_fp12_3, out
    %swap_fp6_hole
    // stack:                                       f, inp, sh(f'f'), square_fp12_3, out
    SWAP6  SWAP13  SWAP6
    // stack:                                       f, square_fp12_3, sh(f'f'), inp, out
    %jump(square_fp6)
square_fp12_3:
    // stack:                                                    ff , sh(f'f'), inp, out
    %add_fp6
    // stack:                                                    ff + sh(f'f'), inp, out
    DUP8
    // stack:                                               out, ff + sh(f'f'), inp, out
    %store_fp6
    // stack:                                                                   inp, out
    %pop2  JUMP
