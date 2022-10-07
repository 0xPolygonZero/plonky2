/// F = f + f'z
/// G = g + g'z
///
/// h + h'z = FG 
///
/// h  = fg + sh(f'g')
/// h' = (f+f')(g+g') - fg - f'g'

mul_Fp12:
    %load_fp6(6)
    %load_fp6(18)
    %dup2_fp6
    %dup2_fp6
    // stack:          g', f', g', f'
    %mul_fp6
    %dup1_fp6
    // stack:      g'f', g'f', g', f'
    %store_fp6_sh(36)
    %store_fp6(42)
    // stack:                  g', f'
    %load_fp6(12)
    // stack:              g , g', f'
    %swap_fp6
    // stack:              g', g , f'
    %dup2_fp6
    // stack:          g , g', g , f'
    %add_fp6
    %swap_fp6
    // stack:          g + g', g , f'
    %swap_fp6
    // stack:          g , g + g', f'
    %load_fp6(0)
    // stack:       f, g , g'+ g , f'
    %mul_fp6
    %store_fp6(48)
    // stack:              g'+ g , f'
    %swap_fp6
    %load_fp6(0)
    %add_fp6
    // stack:           f'+ f, g'+ g
    %mul_fp6
    // stack:            (f+f')(g+g')
    %load_fp6(42)
    %bus_fp6(42)
    // stack: (f+f')(g+g') - f'g'
    %load_fp6(48)
    %swap_fp6
    // stack: (f+f')(g+g') - f'g'     , fg
    %dup2_fp6
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
