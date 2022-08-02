/// Division modulo 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47, the BN254 base field order
/// To replace with more efficient method using non-determinism later.

// Returns y * (x^-1) where the inverse is taken modulo N
%macro moddiv
    // stack: x, y
    %inverse
    // stack: x^-1, y
    %mulmodn
%endmacro

%macro mulmodn
    // stack: x, y
    %bn_base
    // stack: N, x, y
    SWAP2
    // stack: y, x, N
    MULMOD
%endmacro

%macro squaremodn
    // stack: x
    DUP1
    // stack: x, x
    %mulmodn
%endmacro

// Non-deterministically provide the inverse modulo N.
%macro inverse
    // stack: x
    PROVER_INPUT(ff::bn254_base::inverse)
    // stack: x^-1, x
    %stack (inv, x) -> (inv, x, @BN_BASE, inv)
    // stack: x^-1, x, N, x^-1
    MULMOD
    // stack: x^-1 * x, x^-1
    %assert_eq_const(1)
    // stack: x^-1
%endmacro
