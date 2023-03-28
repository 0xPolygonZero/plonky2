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
