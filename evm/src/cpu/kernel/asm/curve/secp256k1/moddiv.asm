/// Division modulo 0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f, the Secp256k1 base field order
/// To replace with more efficient method using non-determinism later.

// Returns y * (x^-1) where the inverse is taken modulo N
%macro moddiv_secp_base
    // stack: x, y
    %inverse_secp_base
    // stack: x^-1, y
    %mulmodn_secp_base
%endmacro

%macro mulmodn_secp_base
    // stack: x, y
    %secp_base
    // stack: N, x, y
    SWAP2
    // stack: y, x, N
    MULMOD
%endmacro

%macro squaremodn_secp_base
    // stack: x
    DUP1
    // stack: x, x
    %mulmodn_secp_base
%endmacro

// Non-deterministically provide the inverse modulo N.
%macro inverse_secp_base
    // stack: x
    PROVER_INPUT(ff::secp256k1_base::inverse)
    // stack: x^-1, x
    %stack (inv, x) -> (inv, x, @SECP_BASE, inv)
    // stack: x^-1, x, N, x^-1
    MULMOD
    // stack: x^-1 * x, x^-1
    %assert_eq_const(1)
    // stack: x^-1
%endmacro
