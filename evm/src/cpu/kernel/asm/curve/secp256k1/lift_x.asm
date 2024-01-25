// Returns y such that (x,y) is on Secp256k1 and y&1 = parity,
// as well as a flag indicating whether such a y exists.
%macro secp_lift_x
    // stack: x, parity
    %cubemodn_secp_base
    // stack: x^3, parity
    PUSH 7
    // stack: 7, x^3, parity
    %addmodn_secp_base
    // stack: x^3+7, x, parity
    DUP1
    // stack: x^3+7, x^3+7, parity
    %sqrt_secp_base_unsafe
    // stack: y, x^3+7, x, parity
    SWAP1
    // stack: x^3+7, y, parity
    DUP2
    // stack: y, x^3+7, y, parity
    %squaremodn_secp_base
    // stack: y^2, x^3+7, y, parity
    EQ
    // stack: sqrtOk, y, parity
    SWAP2
    // stack: parity, y, sqrtOk
    DUP2
    // stack: y, parity, y, sqrtOk
    PUSH 1
    // stack: 1, y, parity, y, sqrtOk
    AND
    // stack: 1 & y, parity, y, sqrtOk
    EQ
    // stack: correctParity, y, sqrtOk
    DUP2
    // stack: y, correctParity, y, sqrtOk
    %secp_base
    // stack: N, y, correctParity, y, sqrtOk
    SUB
    // stack: N - y, correctParity, y, sqrtOk
    SWAP1
    // stack: correctParity, N - y, y, sqrtOk
    %select_bool
    // stack: goody, sqrtOk
%endmacro

%macro cubemodn_secp_base
    // stack: x
    DUP1
    // stack: x, x
    %squaremodn_secp_base
    // stack: x^2, x
    %mulmodn_secp_base
%endmacro

%macro addmodn_secp_base
    // stack: x, y
    %secp_base
    // stack: N, x, y
    SWAP2
    // stack: y, x, N
    ADDMOD
%endmacro

// Non-deterministically provide the square root modulo N.
// Note: The square root is not checked and the macro doesn't panic if `x` is not a square.
%macro sqrt_secp_base_unsafe
    // stack: x
    PROVER_INPUT(ff::secp256k1_base::sqrt)
    // stack: √x, x
    SWAP1
    // stack: x, √x
    POP
    // stack: √x
%endmacro