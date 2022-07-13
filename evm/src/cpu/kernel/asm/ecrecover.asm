global ecrecover:
    JUMPDEST
    // stack: hash, v, r, s, retdest
    %ecrecover_input_check
    // stack: isValid(v,r,s), hash, v, r, s, retdest
    %jumpi(ecrecover_valid_input)
    // stack: hash, v, r, s, retdest
    %pop(4)
    // stack: retdest
    %ecrecover_invalid_input // TODO: Return correct invalid input

ecrecover_valid_input:
    JUMPDEST
    // stack: hash, v, r, s, retdest

// Check if v, r, and s are in correct form.
// Returns r < N & r!=0 & s < N & s!=0 & (v==28 || v==27).
%macro ecrecover_input_check
    // stack: hash, v, r, s, retdest
    DUP2
    // stack: v, hash, v, r, s, retdest
    PUSH 27
    // stack: 27, v, hash, v, r, s, retdest
    EQ
    // stack: v==27, hash, v, r, s, retdest
    DUP3
    // stack: v, v==27, hash, v, r, s, retdest
    PUSH 28
    // stack: 28, v, v==27, hash, v, r, s, retdest
    EQ
    // stack: v==28, v==27, hash, v, r, s, retdest
    OR
    // stack: (v==28 || v==27), hash, v, r, s, retdest
    ISZERO
    // stack: (v==28 || v==27), hash, v, r, s, retdest
    DUP5
    // stack: s, (v==28 || v==27), hash, v, r, s, retdest
    %secp_is_out_of_bounds
    // stack: (s >= N || s==0), (v==28 || v==27), hash, v, r, s, retdest
    DUP5
    // stack: r, (s >= N || s==0), (v==28 || v==27), hash, v, r, s, retdest
    %secp_is_out_of_bounds
    // stack: (r >= N || r==0), (s >= N || s==0), (v==28 || v==27), hash, v, r, s, retdest
    OR
    // stack: (r >= N || r==0 || s >= N || s==0), (v==28 || v==27), hash, v, r, s, retdest
    ISZERO
    // stack: (r < N & r!=0 & s < N & s!=0), (v==28 || v==27), hash, v, r, s, retdest
    AND
    // stack: r < N & r!=0 & s < N & s!=0 & (v==28 || v==27), hash, v, r, s, retdest
%endmacro

%macro secp_is_out_of_bounds
    // stack: x
    DUP1
    // stack: x, x
    ISZERO
    // stack: x==0, x
    SWAP1
    // stack: x, x==0
    %secp_scalar
    // stack: N, x, x==0
    SWAP1
    // stack: x, N, x==0
    LT
    // stack: x < N, x==0
    ISZERO
    // stack: x >= N, x==0
    OR
    // stack: x >= N || x==0
%endmacro

%macro secp_scalar
    PUSH 0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141
%endmacro

// Return (u256::MAX, u256::MAX) which is used to indicate the input was invalid.
%macro ecrecover_invalid_input
    // stack: retdest
    PUSH 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
    // stack: u256::MAX, retdest
    PUSH 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
    // stack: u256::MAX, u256::MAX, retdest
    SWAP2
    // stack: retdest, u256::MAX, u256::MAX
    JUMP
%endmacro
