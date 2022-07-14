global ecrecover:
    JUMPDEST
    // stack: hash, v, r, s, retdest
    %ecrecover_input_check
    // stack: isValid(v,r,s), hash, v, r, s, retdest
    %jumpi(ecrecover_valid_input)
    // stack: hash, v, r, s, retdest
    %pop4
    // stack: retdest
    %ecrecover_invalid_input

// Pseudo-code:
// let P = lift_x(r, recovery_id);
// let r_inv = r.inverse();
// let u1 = s * r_inv;
// let u2 = -hash * r_inv;
// return u1*P + u2*GENERATOR;
ecrecover_valid_input:
    JUMPDEST
    // stack: hash, v, r, s, retdest
    SWAP1
    // stack:  v, hash, r, s, retdest
    DUP3
    // stack:  r, v, hash, r, s, retdest
    %secp_lift_x
    // stack: x, y, hash, r, s, retdest
    SWAP3
    // stack: r, y, hash, x, s, retdest
    %inverse_secp_scalar
    // stack: r^(-1), y, hash, x, s, retdest
    DUP1
    // stack: r^(-1), r^(-1), y, hash, x, s, retdest
    SWAP5
    // stack: s, r^(-1), y, hash, x, r^(-1), retdest
    %mulmodn_secp_scalar
    // stack: u1, y, hash, x, r^(-1), retdest
    PUSH ecrecover_with_first_point
    // stack: ecrecover_with_first_point, u1, y, hash, x, r^(-1), retdest
    SWAP1
    // stack: u1, ecrecover_with_first_point, y, hash, x, r^(-1), retdest
    SWAP2
    // stack: y, ecrecover_with_first_point, u1, hash, x, r^(-1), retdest
    SWAP1
    // stack: ecrecover_with_first_point, y, u1, hash, x, r^(-1), retdest
    SWAP3
    // stack: hash, y, u1, ecrecover_with_first_point, x, r^(-1), retdest
    SWAP4
    // stack: x, y, u1, ecrecover_with_first_point, hash, r^(-1), retdest
    %jump(ec_mul_valid_point_secp)

ecrecover_with_first_point:
    JUMPDEST
    // stack: X, Y, hash, r^(-1), retdest
    %secp_scalar
    // stack: p, X, Y, hash, r^(-1), retdest
    SWAP1
    // stack: X, p, Y, hash, r^(-1), retdest
    SWAP4
    // stack: r^(-1), p, Y, hash, X, retdest
    SWAP2
    // stack: Y, p, r^(-1), hash, X, retdest
    SWAP3
    // stack: hash, p, r^(-1), Y, X, retdest
    MOD
    // stack: hash%p, r^(-1), Y, X, retdest
    %secp_scalar
    // stack: p, hash%p, r^(-1), Y, X, retdest
    SUB
    // stack: -hash, r^(-1), Y, X, retdest
    %mulmodn_secp_scalar
    // stack: u2, Y, X, retdest
    PUSH 8
    // stack: final_hashing, u2, Y, X, retdest
    SWAP3
    // stack: X, u2, Y, final_hashing, retdest
    PUSH 7
    // stack: ec_add_valid_points_secp, X, u2, Y, final_hashing, retdest
    SWAP1
    // stack: X, ec_add_valid_points_secp, u2, Y, final_hashing, retdest
    PUSH 0x79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798 // x-coordinate of generator
    // stack: Gx, X, ec_add_valid_points_secp, u2, Y, final_hashing, retdest
    SWAP1
    // stack: X, Gx, ec_add_valid_points_secp, u2, Y, final_hashing, retdest
    PUSH 0x483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8 // y-coordinate of generator
    // stack: Gy, X, Gx, ec_add_valid_points_secp, u2, Y, final_hashing, retdest
    SWAP1
    // stack: X, Gy, Gx, ec_add_valid_points_secp, u2, Y, final_hashing, retdest
    SWAP4
    // stack: u2, Gy, Gx, ec_add_valid_points_secp, X, Y, final_hashing, retdest
    SWAP2
    // stack: Gx, Gy, u2, ec_add_valid_points_secp, X, Y, final_hashing, retdest
    %jump(ec_mul_valid_point_secp)

// TODO
final_hashing:
    JUMPDEST
    PUSH 0xdeadbeef
    JUMP

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

// Return u256::MAX which is used to indicate the input was invalid.
%macro ecrecover_invalid_input
    // stack: retdest
    PUSH 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
    // stack: u256::MAX, retdest
    SWAP1
    // stack: retdest, u256::MAX
    JUMP
%endmacro
