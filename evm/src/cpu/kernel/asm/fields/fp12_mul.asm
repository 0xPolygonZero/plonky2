global test_mul_Fp12:
    // stack: in0, f, in0', f', in1, g, in1', g', in1, out, in0, out
    %store_fp6
    %store_fp6
    %store_fp6
    %store_fp6
    // stack:               in1, out, in0,                  out
    PUSH return_on_stack
    SWAP3
    // stack:               in0, in1, out, return_on_stack, out
    %jump(mul_Fp12)
return_on_stack:
    // stack: out
    DUP1
    %add_const(6)
    // stack: out', out
    %load_fp6
    %load_fp6
    // stack: h, h'
    %jump(0xdeadbeef)

/// fp6 macros:
///  macro | num | ops | cost
///  -------------------------
///  load  |   8 |  40 |  320
///  store |   5 |  40 |  200
///  dup   |   5 |   6 |   30
///  swap  |   4 |  16 |   64
///  add   |   4 |  16 |   64
///  sub   |   1 |  17 |   17
///  mul   |   3 | 156 |  468
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
///  JUMP  |   1
///
/// TOTAL: 1193


/// F = f + f'z
/// G = g + g'z
///
/// H = h + h'z = FG 
///
/// h  = fg + sh(f'g')
/// h' = (f+f')(g+g') - fg - f'g'
///
/// Note: f, f', g, g' consist of six terms on the stack

global mul_Fp12:
    // stack:                             in0, in1, out
    DUP1  %add_const(6) 
    // stack:                       in0', in0, in1, out  
    %load_fp6
    // stack:                         f', in0, in1, out
    DUP7  %add_const(6)
    // stack:                   in1', f', in0, in1, out
    %load_fp6
    // stack:                     g', f', in0, in1, out
    PUSH post_mul_1
    // stack:         post_mul_1, g', f', in0, in1, out
    %dup_fp6_7
    // stack:     f', post_mul_1, g', f', in0, in1, out
    %dup_fp6_7
    // stack: g', f', post_mul_1, g', f', in0, in1, out
    %jump(mul_fp6)
post_mul_1:
    // stack:               f'g', g'  , f', in0, in1, out
    %dup_fp6_0
    // stack:         f'g', f'g', g'  , f', in0, in1, out
    %store_fp6_sh(36)                                    
    // stack:               f'g', g'  , f', in0, in1, out  {36: sh(f'g')}
    %store_fp6(42)
    // stack:                     g'  , f', in0, in1, out  {36: sh(f'g'), 42: f'g'}
    DUP13
    // stack:                in0, g'  , f', in0, in1, out  {36: sh(f'g'), 42: f'g'}
    DUP15  
    // stack:           in1, in0, g'  , f', in0, in1, out  {36: sh(f'g'), 42: f'g'}
    %load_fp6
    // stack:            g , in0, g'  , f', in0, in1, out  {36: sh(f'g'), 42: f'g'}
    %swap_fp6_hole
    // stack:            g', in0, g   , f', in0, in1, out  {36: sh(f'g'), 42: f'g'}
    dup_fp6_7
    // stack:          g,g', in0, g   , f', in0, in1, out  {36: sh(f'g'), 42: f'g'}
    %add_fp6
    // stack:          g+g', in0, g   , f', in0, in1, out  {36: sh(f'g'), 42: f'g'}
    %swap_fp6_hole
    // stack:             g, in0, g+g', f', in0, in1, out  {36: sh(f'g'), 42: f'g'}
    PUSH post_mul_2
    // stack: post_mul_2, g, in0, g+g', f', in0, in1, out  {36: sh(f'g'), 42: f'g'}
    SWAP7
    // stack: in0, g, post_mul_2, g+g', f', in0, in1, out  {36: sh(f'g'), 42: f'g'}
    %load_fp6
    // stack:   f, g, post_mul_2, g+g', f', in0, in1, out  {36: sh(f'g'), 42: f'g'}
    %jump(mul_fp6)
post_mul_2:    
    // stack:         fg, g+g', f', in0, in1, out  {36: sh(f'g'), 42: f'g'}
    %store_fp6(48)
    // stack:             g+g', f', in0, in1, out  {36: sh(f'g'), 42: f'g', 48: fg}
    %swap_fp6
    // stack:             f', g+g', in0, in1, out  {36: sh(f'g'), 42: f'g', 48: fg}
    PUSH post_mul_3
    // stack: post_mul_3, f', g+g', in0, in1, out  {36: sh(f'g'), 42: f'g', 48: fg}
    SWAP13
    // stack: in0, f', g+g', post_mul_3, in1, out  {36: sh(f'g'), 42: f'g', 48: fg}
    %load_fp6
    // stack:    f,f', g+g', post_mul_3, in1, out  {36: sh(f'g'), 42: f'g', 48: fg}
    %add_fp6
    // stack:    f+f', g+g', post_mul_3, in1, out  {36: sh(f'g'), 42: f'g', 48: fg}
    %jump(mul_fp6)
post_mul_3:
    // stack:                       (f+f')(g+g'), in1, out  {36: sh(f'g'), 42: f'g', 48: fg}
    %load_fp6(48)
    // stack:                   fg, (f+f')(g+g'), in1, out  {36: sh(f'g'), 42: f'g', 48: fg}
    %swap_fp6
    // stack:                   (f+f')(g+g'), fg, in1, out  {36: sh(f'g'), 42: f'g', 48: fg}
    %dup_fp6_6
    // stack:               fg, (f+f')(g+g'), fg, in1, out  {36: sh(f'g'), 42: f'g', 48: fg}
    %load_fp6(42)
    // stack:          f'g',fg, (f+f')(g+g'), fg, in1, out  {36: sh(f'g'), 42: f'g', 48: fg}
    %add_fp6
    // stack:          f'g'+fg, (f+f')(g+g'), fg, in1, out  {36: sh(f'g'), 42: f'g', 48: fg}
    %subr_fp6
    // stack:       (f+f')(g+g') - (f'g'+fg), fg, in1, out  {36: sh(f'g'), 42: f'g', 48: fg}   
    DUP14  add_const(6) 
    // stack: out', (f+f')(g+g') - (f'g'+fg), fg, in1, out  {36: sh(f'g'), 42: f'g', 48: fg}   
    %store_fp6
    // stack:                                 fg, in1, out  {36: sh(f'g'), 42: f'g', 48: fg}
    %load_fp6(36)
    // stack:                      sh(f'g') , fg, in1, out  {36: sh(f'g'), 42: f'g', 48: fg}
    %add_fp6
    // stack:                      sh(f'g') + fg, in1, out  {36: sh(f'g'), 42: f'g', 48: fg}
    DUP8
    // stack:                 out, sh(f'g') + fg, in1, out  {36: sh(f'g'), 42: f'g', 48: fg}
    %store_fp6
    // stack:                                     in1, out  {36: sh(f'g'), 42: f'g', 48: fg}
    %pop2  
    JUMP
