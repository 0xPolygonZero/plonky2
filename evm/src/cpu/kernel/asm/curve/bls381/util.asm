%macro add_fp381
    // stack:         x0, x1, y0, y1
    PROVER_INPUT(sf::bls381_base::add_hi)
    // stack:     z1, x0, x1, y0, y1
    SWAP4
    // stack:     y1, x0, x1, y0, z1
    PROVER_INPUT(sf::bls381_base::add_lo)
    // stack: z0, y1, x0, x1, y0, z1
    SWAP4
    // stack: y0, y1, x0, x1, z0, z1
    %pop4
    // stack:                 z0, z1
%endmacro

%macro sub_fp381
    // stack:         x0, x1, y0, y1
    PROVER_INPUT(sf::bls381_base::sub_hi)
    // stack:     z1, x0, x1, y0, y1
    SWAP4
    // stack:     y1, x0, x1, y0, z1
    PROVER_INPUT(sf::bls381_base::sub_lo)
    // stack: z0, y1, x0, x1, y0, z1
    SWAP4
    // stack: y0, y1, x0, x1, z0, z1
    %pop4
    // stack:                 z0, z1
%endmacro

%macro mul_fp381
    // stack:         x0, x1, y0, y1
    PROVER_INPUT(sf::bls381_base::mul_hi)
    // stack:     z1, x0, x1, y0, y1
    SWAP4
    // stack:     y1, x0, x1, y0, z1
    PROVER_INPUT(sf::bls381_base::mul_lo)
    // stack: z0, y1, x0, x1, y0, z1
    SWAP4
    // stack: y0, y1, x0, x1, z0, z1
    %pop4
    // stack:                 z0, z1
%endmacro

%macro add_fp381_2
    // stack: x_re, x_im, y_re, y_im
    %stack (x_re: 2, x_im: 2, y_re: 2, y_im: 2) -> (y_im, x_im, y_re, x_re)
    // stack: y_im, x_im, y_re, x_re
    %add_fp381
    // stack:       z_im, y_re, x_re
    %stack (z_im: 2, y_re: 2, x_re: 2) -> (x_re, y_re, z_im)
    // stack:       x_re, y_re, z_im
    %add_fp381
    // stack:             z_re, z_im
%endmacro

%macro sub_fp381_2
    // stack: x_re, x_im, y_re, y_im
    %stack (x_re: 2, x_im: 2, y_re: 2, y_im: 2) -> (x_im, y_im, y_re, x_re)
    // stack: x_im, y_im, y_re, x_re
    %sub_fp381
    // stack:       z_im, y_re, x_re
    %stack (z_im: 2, y_re: 2, x_re: 2) -> (x_re, y_re, z_im)
    // stack:       x_re, y_re, z_im
    %sub_fp381
    // stack:             z_re, z_im
%endmacro

// note that {x,y}_{re,im} all take up two stack terms
global mul_fp381_2:
    // stack:                          x_re, x_im, y_re, y_im, jumpdest
    DUP4
    DUP4
    // stack:                    x_im, x_re, x_im, y_re, y_im, jumpdest
    DUP8
    DUP8
    // stack:              y_re, x_im, x_re, x_im, y_re, y_im, jumpdest
    DUP12
    DUP12
    // stack:        y_im, y_re, x_im, x_re, x_im, y_re, y_im, jumpdest
    DUP8
    DUP8
    // stack: x_re , y_im, y_re, x_im, x_re, x_im, y_re, y_im, jumpdest
    %mul_fp381
    // stack: x_re * y_im, y_re, x_im, x_re, x_im, y_re, y_im, jumpdest
    %stack (v: 2, y_re: 2, x_im: 2) ->  (x_im, y_re, v)
    // stack:  x_im , y_re, x_re*y_im, x_re, x_im, y_re, y_im, jumpdest
    %mul_fp381
    // stack:  x_im * y_re, x_re*y_im, x_re, x_im, y_re, y_im, jumpdest
    %add_fp381
    // stack:                    z_im, x_re, x_im, y_re, y_im, jumpdest
    %stack (z_im: 2, x_re: 2, x_im: 2, y_re: 2, y_im: 2) -> (x_im, y_im, y_re, x_re, z_im)
    // stack:                   x_im , y_im, y_re, x_re, z_im, jumpdest
    %mul_fp381
    // stack:                   x_im * y_im, y_re, x_re, z_im, jumpdest
    %stack (v: 2, y_re: 2, x_re: 2) -> (x_re, y_re, v)
    // stack:                    x_re , y_re, x_im*y_im, z_im, jumpdest
    %mul_fp381
    // stack:                    x_re * y_re, x_im*y_im, z_im, jumpdest
    %sub_fp381
    // stack:                                      z_re, z_im, jumpdest
    %stack (z_re: 2, z_im: 2, jumpdest) -> (jumpdest, z_re, z_im)
    JUMP
