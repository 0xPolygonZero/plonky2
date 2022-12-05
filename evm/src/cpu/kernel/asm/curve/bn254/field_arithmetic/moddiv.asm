/// Division modulo 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47, the BN254 base field order

// Returns y * (x^-1) where the inverse is taken modulo N
%macro moddiv
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
