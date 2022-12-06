/// Division modulo the BN254 prime

// Returns y * (x^-1) where the inverse is taken modulo N
%macro divfp254
    // stack: x   , y
    %inverse
    // stack: x^-1, y
    MULFP254
%endmacro

// Non-deterministically provide the inverse modulo N.
%macro inverse
    // stack: x
    PROVER_INPUT(ff::bn254_base::inverse)
    // stack: x^-1 , x
    SWAP1  DUP2
    // stack: x^-1 , x, x^-1
    MULFP254
    // stack: x^-1 * x, x^-1
    %assert_eq_const(1)
    // stack:           x^-1
%endmacro
