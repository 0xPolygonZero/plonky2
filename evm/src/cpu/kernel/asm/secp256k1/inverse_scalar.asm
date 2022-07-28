/// Division modulo 0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141, the Secp256k1 scalar field order
/// To replace with more efficient method using non-determinism later.

%macro mulmodn_secp_scalar
    // stack: x, y
    %secp_scalar
    // stack: N, x, y
    SWAP2
    // stack: y, x, N
    MULMOD
%endmacro

%macro squaremodn_secp_scalar
    // stack: x
    DUP1
    // stack: x, x
    %mulmodn_secp_scalar
%endmacro

// Non-deterministically provide the inverse modulo N.
%macro inverse_secp_scalar
    // stack: x
    PROVER_INPUT(ff::secp256k1_scalar::inverse)
    // stack: x^-1, x
    %stack (inv, x) -> (inv, x, @SECP_SCALAR, inv)
    // stack: x^-1, x, N, x^-1
    MULMOD
    // stack: x^-1 * x, x^-1
    %assert_eq_const(1)
    // stack: x^-1
%endmacro
