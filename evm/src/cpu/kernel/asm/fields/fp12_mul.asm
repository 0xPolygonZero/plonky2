global test_mul_Fp12:
    // stack: f, f', g, g', in2, out, in1
    %store_fp6(0)
    %store_fp6(6)
    %store_fp6(12)
    %store_fp6(18)
    // stack:               in2, out, in1
    PUSH return_on_stack
    SWAP3
    // stack:               in1, in2, out, return_on_stack
    %jump(mul_Fp12)
return_on_stack:
    // stack:
    %load_fp6(30)
    %load_fp6(24)
    // stack: h, h'
    %jump(0xdeadbeef)

/// fp6 macros:
///  macro | num | ops | cost
///  -------------------------
///  load  |   8 |  40 |  320
///  store |   5 |  40 |  200
///  dup   |   5 |   6 |   30
///  swap  |   4 |  16 |   64
///  add   |   3 |  16 |   48
///  sub   |   2 |  17 |   34
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
/// TOTAL: 1194


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
    // stack:                             in1, in2, out
    DUP1  
    %add_const(6)  
    %load_fp6
    // stack:                         f', in1, in2, out
    DUP7  
    %add_const(6)  
    %load_fp6
    // stack:                     g', f', in1, in2, out
    PUSH post_mul_1
    %dup_fp6_7
    // stack:     f', post_mul_1, g', f', in1, in2, out
    %dup_fp6_7
    // stack: g', f', post_mul_1, g', f', in1, in2, out
    %jump(mul_fp6)
post_mul_1:
    // stack:             f'g', g'  , f', in1, in2, out
    %dup_fp6_0
    // stack:       f'g', f'g', g'  , f', in1, in2, out
    %store_fp6_sh(36)
    // stack:             f'g', g'  , f', in1, in2, out
    %store_fp6(42)
    // stack:                   g'  , f', in1, in2, out
    DUP13
    // stack:              in1, g'  , f', in1, in2, out
    DUP15  
    %load_fp6
    // stack:          g , in1, g'  , f', in1, in2, out
    %swap_fp6_hole
    // stack:          g', in1, g   , f', in1, in2, out
    dup_fp6_7
    // stack:        g,g', in1, g   , f', in1, in2, out
    %add_fp6
    // stack:        g+g', in1, g   , f', in1, in2, out
    %swap_fp6_hole
    // stack:           g, in1, g+g', f', in1, in2, out
    PUSH post_mul_2
    SWAP7
    %load_fp6
    // stack: f, g, post_mul_2, g+g', f', in1, in2, out
    %jump(mul_fp6)
post_mul_2:    
    // stack:      fg, g+g', f', in1, in2, out
    %store_fp6(48)
    // stack:          g+g', f', in1, in2, out
    %swap_fp6
    // stack:          f', g+g', in1, in2, out
    PUSH post_mul_3
    SWAP13  
    %load_fp6
    // stack: f,f', g+g', post_mul_3, in2, out
    %add_fp6
    // stack: f+f', g+g', post_mul_3, in2, out
    %jump(mul_fp6)
post_mul_3:
    // stack:             (f+f')(g+g'), in2, out
    %load_fp6(42)
    // stack:       f'g', (f+f')(g+g'), in2, out
    %subr_fp6
    // stack:      (f+f')(g+g') - f'g', in2, out
    %load_fp6(48)
    // stack:  fg, (f+f')(g+g') - f'g', in2, out
    %swap_fp6
    // stack:      (f+f')(g+g') - f'g', fg, in2, out
    %dup_fp6_6
    // stack:  fg, (f+f')(g+g') - f'g', fg, in2, out
    %subr_fp6
    // stack: (f+f')(g+g') - f'g' - fg, fg, in2, out
    DUP14  
    add_const(6)  
    %store_fp6
    // stack:                           fg, in2, out
    %load_fp6(36)
    // stack:                sh(f'g') , fg, in2, out
    %add_fp6
    // stack:                sh(f'g') + fg, in2, out
    DUP8  
    %store_fp6
    // stack:                               in2, out
    %pop2  
    JUMP
