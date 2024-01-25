///////////////////////////////////////
///// GENERAL FP12 MULTIPLICATION /////
///////////////////////////////////////

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

global mul_fp254_12:
    // stack:                                   inA, inB, out 
    DUP1  
    %add_const(6) 
    // stack:                             inA', inA, inB, out 
    %load_fp254_6
    // stack:                               f', inA, inB, out 
    DUP8  
    %add_const(6)
    // stack:                         inB', f', inA, inB, out 
    %load_fp254_6
    // stack:                           g', f', inA, inB, out 
    PUSH mul_fp254_12_1
    // stack:           mul_fp254_12_1, g', f', inA, inB, out 
    %dup_fp254_6_7
    // stack:       f', mul_fp254_12_1, g', f', inA, inB, out 
    %dup_fp254_6_7
    // stack:   g', f', mul_fp254_12_1, g', f', inA, inB, out 
    %jump(mul_fp254_6)
mul_fp254_12_1:
    // stack:                   f'g', g'  , f', inA, inB, out 
    %dup_fp254_6_0
    // stack:             f'g', f'g', g'  , f', inA, inB, out 
    %store_fp254_6_sh(60)                                    
    // stack:                   f'g', g'  , f', inA, inB, out  {60: sh(f'g')}
    %store_fp254_6(66)
    // stack:                         g'  , f', inA, inB, out  {60: sh(f'g'), 66: f'g'}
    DUP13
    // stack:                    inA, g'  , f', inA, inB, out  {60: sh(f'g'), 66: f'g'}
    DUP15  
    // stack:               inB, inA, g'  , f', inA, inB, out  {60: sh(f'g'), 66: f'g'}
    %load_fp254_6
    // stack:                g , inA, g'  , f', inA, inB, out  {60: sh(f'g'), 66: f'g'}
    %stack (f: 6, x, g: 6) -> (g, x, f)
    // stack:                g', inA, g   , f', inA, inB, out  {60: sh(f'g'), 66: f'g'}
    %dup_fp254_6_7
    // stack:              g,g', inA, g   , f', inA, inB, out  {60: sh(f'g'), 66: f'g'}
    %add_fp254_6
    // stack:              g+g', inA, g   , f', inA, inB, out  {60: sh(f'g'), 66: f'g'}
    %stack (f: 6, x, g: 6) -> (g, x, f)
    // stack:                 g, inA, g+g', f', inA, inB, out  {60: sh(f'g'), 66: f'g'}
    PUSH mul_fp254_12_2
    // stack: mul_fp254_12_2, g, inA, g+g', f', inA, inB, out  {60: sh(f'g'), 66: f'g'}
    SWAP7
    // stack: inA, g, mul_fp254_12_2, g+g', f', inA, inB, out  {60: sh(f'g'), 66: f'g'}
    %load_fp254_6
    // stack:   f, g, mul_fp254_12_2, g+g', f', inA, inB, out  {60: sh(f'g'), 66: f'g'}
    %jump(mul_fp254_6)
mul_fp254_12_2:    
    // stack:                     fg, g+g', f', inA, inB, out  {60: sh(f'g'), 66: f'g'}
    %store_fp254_6(72)
    // stack:                         g+g', f', inA, inB, out  {60: sh(f'g'), 66: f'g', 72: fg}
    %stack (x: 6, y: 6) -> (y, x)
    // stack:                         f', g+g', inA, inB, out  {60: sh(f'g'), 66: f'g', 72: fg}
    PUSH mul_fp254_12_3
    // stack:         mul_fp254_12_3, f', g+g', inA, inB, out  {60: sh(f'g'), 66: f'g', 72: fg}
    SWAP13
    // stack:         inA, f', g+g', mul_fp254_12_3, inB, out  {60: sh(f'g'), 66: f'g', 72: fg}
    %load_fp254_6
    // stack:            f,f', g+g', mul_fp254_12_3, inB, out  {60: sh(f'g'), 66: f'g', 72: fg}
    %add_fp254_6
    // stack:            f+f', g+g', mul_fp254_12_3, inB, out  {60: sh(f'g'), 66: f'g', 72: fg}
    %jump(mul_fp254_6)
mul_fp254_12_3:
    // stack:                          (f+f')(g+g'), inB, out  {60: sh(f'g'), 66: f'g', 72: fg}
    %load_fp254_6(72)
    // stack:                      fg, (f+f')(g+g'), inB, out  {60: sh(f'g'), 66: f'g', 72: fg}
    %stack (x: 6, y: 6) -> (y, x)
    // stack:                      (f+f')(g+g'), fg, inB, out  {60: sh(f'g'), 66: f'g', 72: fg}
    %dup_fp254_6_6
    // stack:                  fg, (f+f')(g+g'), fg, inB, out  {60: sh(f'g'), 66: f'g', 72: fg}
    %load_fp254_6(66)
    // stack:             f'g',fg, (f+f')(g+g'), fg, inB, out  {60: sh(f'g'), 66: f'g', 72: fg}
    %add_fp254_6
    // stack:             f'g'+fg, (f+f')(g+g'), fg, inB, out  {60: sh(f'g'), 66: f'g', 72: fg}
    %subr_fp254_6
    // stack:          (f+f')(g+g') - (f'g'+fg), fg, inB, out  {60: sh(f'g'), 66: f'g', 72: fg}   
    DUP14  
    %add_const(6) 
    // stack:    out', (f+f')(g+g') - (f'g'+fg), fg, inB, out  {60: sh(f'g'), 66: f'g', 72: fg}   
    %store_fp254_6
    // stack:                                    fg, inB, out  {60: sh(f'g'), 66: f'g', 72: fg}
    %load_fp254_6(60)
    // stack:                         sh(f'g') , fg, inB, out  {60: sh(f'g'), 66: f'g', 72: fg}
    %add_fp254_6
    // stack:                         sh(f'g') + fg, inB, out  {60: sh(f'g'), 66: f'g', 72: fg}
    DUP8
    // stack:                    out, sh(f'g') + fg, inB, out  {60: sh(f'g'), 66: f'g', 72: fg}
    %store_fp254_6
    // stack:                                        inB, out  {60: sh(f'g'), 66: f'g', 72: fg}
    %pop2  
    JUMP


//////////////////////////////////////
///// SPARSE FP12 MULTIPLICATION /////
//////////////////////////////////////

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

global mul_fp254_12_sparse:
    // stack:                                                                    inA, inB, out
    DUP1  
    %add_const(6)
    // stack:                                                              inA', inA, inB, out
    %load_fp254_6
    // stack:                                                                f', inA, inB, out
    DUP8 
    // stack:                                                           inB, f', inA, inB, out
    DUP8
    // stack:                                                      inA, inB, f', inA, inB, out
    %load_fp254_6
    // stack:                                                        f, inB, f', inA, inB, out
    DUP16
    // stack:                                                   out, f, inB, f', inA, inB, out
    %dup_fp254_6_8 
    // stack:                                               f', out, f, inB, f', inA, inB, out
    DUP14
    // stack:                                          inB, f', out, f, inB, f', inA, inB, out
    %dup_fp254_6_8
    // stack:                                       f, inB, f', out, f, inB, f', inA, inB, out
    DUP7
    // stack:                                  inB, f, inB, f', out, f, inB, f', inA, inB, out
    %dup_fp254_6_8
    // stack:                              f', inB, f, inB, f', out, f, inB, f', inA, inB, out
    %dup_fp254_6_7
    // stack:                           f, f', inB, f, inB, f', out, f, inB, f', inA, inB, out
    DUP13 
    // stack:                      inB, f, f', inB, f, inB, f', out, f, inB, f', inA, inB, out
    %mload_bn254_pairing
    // stack:                      g0 , f, f', inB, f, inB, f', out, f, inB, f', inA, inB, out
    %scale_re_fp254_6
    // stack:                      g0 * f, f', inB, f, inB, f', out, f, inB, f', inA, inB, out
    %stack (x: 6, y: 6) -> (y, x)
    // stack:                    f'  , g0 * f, inB, f, inB, f', out, f, inB, f', inA, inB, out
    DUP13
    %add_const(8)
    // stack:           inB2,    f'  , g0 * f, inB, f, inB, f', out, f, inB, f', inA, inB, out
    %load_fp254_2
    // stack:           G2  ,    f'  , g0 * f, inB, f, inB, f', out, f, inB, f', inA, inB, out
    %scale_fp254_6_sh2
    // stack:           G2 * sh2(f') , g0 * f, inB, f, inB, f', out, f, inB, f', inA, inB, out
    %add_fp254_6
    // stack:           G2 * sh2(f') + g0 * f, inB, f, inB, f', out, f, inB, f', inA, inB, out
    %stack (f: 6, x, g: 6) -> (g, x, f)
    // stack:          f , inB, G2 * sh2(f') + g0 * f, inB, f', out, f, inB, f', inA, inB, out
    DUP7  %add_const(2)
    // stack: inB1,    f , inB, G2 * sh2(f') + g0 * f, inB, f', out, f, inB, f', inA, inB, out
    %load_fp254_2
    // stack:  G1 ,    f , inB, G2 * sh2(f') + g0 * f, inB, f', out, f, inB, f', inA, inB, out
    %scale_fp254_6_sh
    // stack:  G1 * sh(f), inB, G2 * sh2(f') + g0 * f, inB, f', out, f, inB, f', inA, inB, out
    %add_fp254_6_hole
    // stack:      G1 * sh(f) + G2 * sh2(f') + g0 * f, inB, f', out, f, inB, f', inA, inB, out
    DUP14
    // stack: out, G1 * sh(f) + G2 * sh2(f') + g0 * f, inB, f', out, f, inB, f', inA, inB, out
    %store_fp254_6
    // stack:                                          inB, f', out, f, inB, f', inA, inB, out
    %mload_bn254_pairing
    // stack:                                          g0 , f', out, f, inB, f', inA, inB, out
    %scale_re_fp254_6
    // stack:                                          g0 * f', out, f, inB, f', inA, inB, out
    %stack (f: 6, x, g: 6) -> (g, x, f)
    // stack:                                        f  , out, g0 * f', inB, f', inA, inB, out
    DUP14
    %add_const(8)
    // stack:                               inB2,    f  , out, g0 * f', inB, f', inA, inB, out
    %load_fp254_2
    // stack:                                G2 ,    f  , out, g0 * f', inB, f', inA, inB, out
    %scale_fp254_6_sh
    // stack:                                G2 * sh(f) , out, g0 * f', inB, f', inA, inB, out
    %add_fp254_6_hole
    // stack:                                     G2 * sh(f) + g0 * f', inB, f', inA, inB, out
    %stack (f: 6, x, g: 6) -> (g, x, f)
    // stack:                                    f' , inB, G2 * sh(f) + g0 * f', inA, inB, out
    DUP7
    %add_const(2)
    // stack:                           inB1,    f' , inB, G2 * sh(f) + g0 * f', inA, inB, out
    %load_fp254_2
    // stack:                            G1 ,    f' , inB, G2 * sh(f) + g0 * f', inA, inB, out
    %scale_fp254_6_sh
    // stack:                            G1 * sh(f'), inB, G2 * sh(f) + g0 * f', inA, inB, out
    %add_fp254_6_hole
    // stack:                                G1 * sh(f') + G2 * sh(f) + g0 * f', inA, inB, out
    DUP9
    %add_const(6)
    // stack:                          out', G1 * sh(f') + G2 * sh(f) + g0 * f', inA, inB, out
    %store_fp254_6
    // stack:                                                                    inA, inB, out
    %pop3
    JUMP


/////////////////////////
///// FP12 SQUARING /////
/////////////////////////

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

global square_fp254_12:
    // stack:                                                                               inp, out
    DUP1
    // stack:                                                                          inp, inp, out
    %load_fp254_6 
    // stack:                                                                            f, inp, out
    PUSH square_fp254_12_3
    // stack:                                                         square_fp254_12_3, f, inp, out
    SWAP7
    // stack:                                                         inp, f, square_fp254_12_3, out
    PUSH square_fp254_12_2
    // stack:                                      square_fp254_12_2, inp, f, square_fp254_12_3, out 
    %dup_fp254_6_2
    // stack:                                  f , square_fp254_12_2, inp, f, square_fp254_12_3, out
    DUP16
    %add_const(6)
    // stack:                            out', f , square_fp254_12_2, inp, f, square_fp254_12_3, out
    PUSH square_fp254_12_1
    // stack:         square_fp254_12_1, out', f , square_fp254_12_2, inp, f, square_fp254_12_3, out
    DUP10
    %add_const(6)
    // stack:   inp', square_fp254_12_1, out', f , square_fp254_12_2, inp, f, square_fp254_12_3, out
    %load_fp254_6
    // stack:     f', square_fp254_12_1, out', f , square_fp254_12_2, inp, f, square_fp254_12_3, out
    %stack (f: 6, x: 2, g: 6) -> (g, x, f)
    // stack:     f , square_fp254_12_1, out', f', square_fp254_12_2, inp, f, square_fp254_12_3, out
    %dup_fp254_6_8
    // stack: f', f , square_fp254_12_1, out', f', square_fp254_12_2, inp, f, square_fp254_12_3, out
    %jump(mul_fp254_6)
square_fp254_12_1:
    // stack:                       f'f, out', f', square_fp254_12_2, inp, f, square_fp254_12_3, out
    DUP7
    // stack:                 out', f'f, out', f', square_fp254_12_2, inp, f, square_fp254_12_3, out
    %store_fp254_6_double
    // stack:                            out', f', square_fp254_12_2, inp, f, square_fp254_12_3, out
    POP
    // stack:                                  f', square_fp254_12_2, inp, f, square_fp254_12_3, out
    %jump(square_fp254_6)
square_fp254_12_2:
    // stack:                                                   f'f', inp, f, square_fp254_12_3, out
    %sh_fp254_6
    // stack:                                               sh(f'f'), inp, f, square_fp254_12_3, out
    %stack (f: 6, x, g: 6) -> (g, x, f)
    // stack:                                               f, inp, sh(f'f'), square_fp254_12_3, out
    SWAP6
    SWAP13
    SWAP6
    // stack:                                               f, square_fp254_12_3, sh(f'f'), inp, out
    %jump(square_fp254_6)
square_fp254_12_3:
    // stack:                                                                ff , sh(f'f'), inp, out
    %add_fp254_6
    // stack:                                                                ff + sh(f'f'), inp, out
    DUP8
    // stack:                                                           out, ff + sh(f'f'), inp, out
    %store_fp254_6
    // stack:                                                                               inp, out
    %pop2
    JUMP
