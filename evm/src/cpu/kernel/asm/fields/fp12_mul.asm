/// Note: uncomment this to test

/// global test_mul_Fp12:
///     // stack:      f, in0 , f', g, in1 , g', in1, out, in0,       out
///     DUP7
///     // stack: in0, f, in0 , f', g, in1 , g', in1, out, in0,       out
///     %store_fp6
///     // stack:         in0 , f', g, in1 , g', in1, out, in0,       out
///     %add_const(6)
///     // stack:         in0', f', g, in1 , g', in1, out, in0,       out
///     %store_fp6
///     // stack:                   g, in1 , g', in1, out, in0,       out
///     DUP7
///     // stack:              in1, g, in1 , g', in1, out, in0,       out
///     %store_fp6
///     // stack:                      in1 , g', in1, out, in0,       out
///     %add_const(6)
///     // stack:                      in1', g', in1, out, in0,       out
///     %store_fp6
///     // stack:                                in1, out, in0,       out
///     PUSH ret_stack
///     // stack:                     ret_stack, in1, out, in0,       out
///     SWAP3
///     // stack:                           in0, in1, out, ret_stack, out
///     %jump(mul_Fp12)
/// ret_stack:
///     // stack:          out
///     DUP1  %add_const(6)
///     // stack:    out', out
///     %load_fp6
///     // stack:      h', out
///     DUP7
///     // stack: out, h', out
///     %load_fp6
///     // stack:   h, h', out
///     %jump(0xdeadbeef)


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
///  JUMP  |   1
///
/// TOTAL: 1196

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
///     {in0: f, in0: f', in1: g, in1':g', out: h, out': h'}
///
/// f, f', g, g' consist of six elements on the stack

global mul_Fp12:
    // stack:                                in0, in1, out 
    DUP1  %add_const(6) 
    // stack:                          in0', in0, in1, out 
    %load_fp6
    // stack:                            f', in0, in1, out 
    DUP8  %add_const(6)
    // stack:                      in1', f', in0, in1, out 
    %load_fp6
    // stack:                        g', f', in0, in1, out 
    PUSH ret_1
    // stack:                 ret_1, g', f', in0, in1, out 
    %dup_fp6_7
    // stack:             f', ret_1, g', f', in0, in1, out 
    %dup_fp6_7
    // stack:         g', f', ret_1, g', f', in0, in1, out 
    %jump(mul_fp6)
ret_1:
    // stack:                f'g', g'  , f', in0, in1, out 
    %dup_fp6_0
    // stack:          f'g', f'g', g'  , f', in0, in1, out 
    %store_fp6_sh(100)                                    
    // stack:                f'g', g'  , f', in0, in1, out  {100: sh(f'g')}
    %store_fp6(106)
    // stack:                      g'  , f', in0, in1, out  {100: sh(f'g'), 106: f'g'}
    DUP13
    // stack:                 in0, g'  , f', in0, in1, out  {100: sh(f'g'), 106: f'g'}
    DUP15  
    // stack:            in1, in0, g'  , f', in0, in1, out  {100: sh(f'g'), 106: f'g'}
    %load_fp6
    // stack:             g , in0, g'  , f', in0, in1, out  {100: sh(f'g'), 106: f'g'}
    %swap_fp6_hole
    // stack:             g', in0, g   , f', in0, in1, out  {100: sh(f'g'), 106: f'g'}
    %dup_fp6_7
    // stack:           g,g', in0, g   , f', in0, in1, out  {100: sh(f'g'), 106: f'g'}
    %add_fp6
    // stack:           g+g', in0, g   , f', in0, in1, out  {100: sh(f'g'), 106: f'g'}
    %swap_fp6_hole
    // stack:              g, in0, g+g', f', in0, in1, out  {100: sh(f'g'), 106: f'g'}
    PUSH ret_2
    // stack:       ret_2, g, in0, g+g', f', in0, in1, out  {100: sh(f'g'), 106: f'g'}
    SWAP7
    // stack:       in0, g, ret_2, g+g', f', in0, in1, out  {100: sh(f'g'), 106: f'g'}
    %load_fp6
    // stack:         f, g, ret_2, g+g', f', in0, in1, out  {100: sh(f'g'), 106: f'g'}
    %jump(mul_fp6)
ret_2:    
    // stack:                  fg, g+g', f', in0, in1, out  {100: sh(f'g'), 106: f'g'}
    %store_fp6(112)
    // stack:                      g+g', f', in0, in1, out  {100: sh(f'g'), 106: f'g', 112: fg}
    %swap_fp6
    // stack:                      f', g+g', in0, in1, out  {100: sh(f'g'), 106: f'g', 112: fg}
    PUSH ret_3
    // stack:               ret_3, f', g+g', in0, in1, out  {100: sh(f'g'), 106: f'g', 112: fg}
    SWAP13
    // stack:               in0, f', g+g', ret_3, in1, out  {100: sh(f'g'), 106: f'g', 112: fg}
    %load_fp6
    // stack:                  f,f', g+g', ret_3, in1, out  {100: sh(f'g'), 106: f'g', 112: fg}
    %add_fp6
    // stack:                  f+f', g+g', ret_3, in1, out  {100: sh(f'g'), 106: f'g', 112: fg}
    %jump(mul_fp6)
ret_3:
    // stack:                       (f+f')(g+g'), in1, out  {100: sh(f'g'), 106: f'g', 112: fg}
    %load_fp6(112)
    // stack:                   fg, (f+f')(g+g'), in1, out  {100: sh(f'g'), 106: f'g', 112: fg}
    %swap_fp6
    // stack:                   (f+f')(g+g'), fg, in1, out  {100: sh(f'g'), 106: f'g', 112: fg}
    %dup_fp6_6
    // stack:               fg, (f+f')(g+g'), fg, in1, out  {100: sh(f'g'), 106: f'g', 112: fg}
    %load_fp6(106)
    // stack:          f'g',fg, (f+f')(g+g'), fg, in1, out  {100: sh(f'g'), 106: f'g', 112: fg}
    %add_fp6
    // stack:          f'g'+fg, (f+f')(g+g'), fg, in1, out  {100: sh(f'g'), 106: f'g', 112: fg}
    %subr_fp6
    // stack:       (f+f')(g+g') - (f'g'+fg), fg, in1, out  {100: sh(f'g'), 106: f'g', 112: fg}   
    DUP14  %add_const(6) 
    // stack: out', (f+f')(g+g') - (f'g'+fg), fg, in1, out  {100: sh(f'g'), 106: f'g', 112: fg}   
    %store_fp6
    // stack:                                 fg, in1, out  {100: sh(f'g'), 106: f'g', 112: fg}
    %load_fp6(100)
    // stack:                      sh(f'g') , fg, in1, out  {100: sh(f'g'), 106: f'g', 112: fg}
    %add_fp6
    // stack:                      sh(f'g') + fg, in1, out  {100: sh(f'g'), 106: f'g', 112: fg}
    DUP8
    // stack:                 out, sh(f'g') + fg, in1, out  {100: sh(f'g'), 106: f'g', 112: fg}
    %store_fp6
    // stack:                                     in1, out  {100: sh(f'g'), 106: f'g', 112: fg}
    %pop2  
    JUMP
