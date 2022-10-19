global test_mul_Fp12:
    // stack: f, f', g, g'
    %store_fp6(0)
    %store_fp6(6)
    %store_fp6(12)
    %store_fp6(18)
    PUSH return_on_stack
    // stack: return_on_stack
    %jump(mul_Fp12)
return_on_stack:
    // stack:
    %load_fp6(30)
    %load_fp6(24)
    // stack: h, h'
    %jump(0xdeadbeef)


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
///  jump  |   1 |   1 |    1
///
/// TOTAL: 1174


/// F = f + f'z
/// G = g + g'z
///
/// H = h + h'z = FG 
///
/// h  = fg + sh(f'g')
/// h' = (f+f')(g+g') - fg - f'g'
///
/// Note: each symbol in the stack comments consists of six words

global mul_Fp12:
    %load_fp6(6)
    // stack:                     f'
    %load_fp6(18)
    // stack:                 g', f'
    %dup2_fp6
    // stack:             f', g', f'
    %dup2_fp6
    // stack:         g', f', g', f'
    %mul_fp6
    // stack:           f'g', g', f'
    %dup1_fp6
    // stack:     f'g', f'g', g', f'
    %store_fp6_sh(36)
    // stack:           f'g', g', f'
    %store_fp6(42)
    // stack:                 g', f'
    %load_fp6(12)
    // stack:             g , g', f'
    %swap_fp6
    // stack:             g', g , f'
    %dup2_fp6
    // stack:          g, g', g , f'
    %add_fp6
    // stack:           g+g', g , f'
    %swap_fp6
    // stack:            g, g+g', f'
    %load_fp6(0)
    // stack:        f, g , g+g', f'
    %mul_fp6
    // stack:          fg , g+g', f'
    %store_fp6(48)
    // stack:               g+g', f'
    %swap_fp6
    // stack:               f', g+g'
    %load_fp6(0)
    // stack:             f,f', g+g'
    %add_fp6
    // stack:             f+f', g+g'
    %mul_fp6
    // stack:            (f+f')(g+g')
    %load_fp6(42)
    // stack:      f'g', (f+f')(g+g')
    %bus_fp6
    // stack:      (f+f')(g+g') - f'g'
    %load_fp6(48)
    // stack:  fg, (f+f')(g+g') - f'g'
    %swap_fp6
    // stack:      (f+f')(g+g') - f'g', fg
    %dup2_fp6
    // stack:  fg, (f+f')(g+g') - f'g', fg
    %bus_fp6
    // stack: (f+f')(g+g') - f'g' - fg, fg
    %store_fp6(30)
    // stack:                           fg
    %load_fp6(36)
    // stack:                sh(f'g') , fg
    %add_fp6
    // stack:                sh(f'g') + fg
    %store_fp6(24)
    JUMP
