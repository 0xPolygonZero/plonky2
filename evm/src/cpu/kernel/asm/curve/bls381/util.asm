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

global test_add_fp381:
    %add_fp381
    %jump(0xdeadbeef)

global test_mul_fp381:
    %mul_fp381
    %jump(0xdeadbeef)

global test_sub_fp381:
    %sub_fp381
    %jump(0xdeadbeef)


global add_fp381_2:
    // stack:         x: 2, x_: 2, y: 2, y_: 2
    %stack (x: 2, x_: 2, y: 2, y_: 2) -> (y_, x_, y, x)
    // stack:         y_: 2, x_: 2, y: 2, x: 2
    %add_fp381
    // stack:                z_: 2, y: 2, x: 2
    %stack (z_: 2, y: 2, x: 2) -> (x, y, z_)
    // stack:                x: 2, y: 2, z_: 2
    %add_fp381
    // stack:                      z: 2, z_: 2
    %jump(0xdeadbeef)

global mul_fp381_2:
    // stack:             a, b, c, d
    DUP4
    DUP4
    // stack:          b, a, b, c, d
    DUP8
    DUP8
    // stack:       c, b, a, b, c, d
    DUP12
    DUP12
    // stack:    d, c, b, a, b, c, d
    DUP8
    DUP8
    // stack: a, d, c, b, a, b, c, d

    // stack: a, d, c, b, a, b, c, d
    %mul_fp381
    // stack:   ad, c, b, a, b, c, d
    %stack (ad: 2, c: 2, b: 2) ->  (b, c, ad)
    // stack:   b, c, ad, a, b, c, d
    %mul_fp381
    // stack:     bc, ad, a, b, c, d
    %add_fp381
    // stack:       z_im, a, b, c, d
    %stack (z_im: 2, a: 2, b: 2, c: 2, d: 2) -> (b, d, c, a, z_im)
    // stack:       b, d, c, a, z_im
    %mul_fp381
    // stack:         bd, c, a, z_im
    %stack (bd: 2, c: 2, a: 2) -> (a, c, bd)
    // stack:         a, c, bd, z_im
    %mul_fp381
    // stack:           ac, bd, z_im
    %sub_fp381
    // stack:             z_re, z_im
    %jump(0xdeadbeef)

global sub_fp381_2:
    // stack:         x: 2, x_: 2, y: 2, y_: 2
    %stack (x: 2, x_: 2, y: 2, y_: 2) -> (x_, y_, y, x)
    // stack:         x_: 2, y_: 2, y: 2, x: 2
    %sub_fp381
    // stack:                z_: 2, y: 2, x: 2
    %stack (z_: 2, y: 2, x: 2) -> (x, y, z_)
    // stack:                x: 2, y: 2, z_: 2
    %sub_fp381
    // stack:                      z: 2, z_: 2
    %jump(0xdeadbeef)
