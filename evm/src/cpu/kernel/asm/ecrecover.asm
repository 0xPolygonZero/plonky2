// ecrecover precompile.
global ecrecover:
    JUMPDEST
    // stack: hash, v, r, s, retdest

    // Check if inputs are valid.
    %ecrecover_input_check
    // stack: isValid(v,r,s), hash, v, r, s, retdest

    // Lift r to an elliptic curve point if possible.
    SWAP2
    // stack: v, hash, isValid(v,r,s), r, s, retdest
    DUP4
    // stack: r, v, hash, isValid(v,r,s), r, s, retdest

    // Compute v-27 which gives the parity of the y-coordinate of the lifted point.
    SWAP1
    // stack: v, r, hash, isValid(v,r,s), r, s, retdest
    PUSH 27
    // stack: 27, v, r, hash, isValid(v,r,s), r, s, retdest
    SWAP1
    // stack: v, 27, r, hash, isValid(v,r,s), r, s, retdest
    SUB
    // stack: v - 27, r, hash, isValid(v,r,s), r, s, retdest
    SWAP1
    // stack: r, v - 27, hash, isValid(v,r,s), r, s, retdest
    %secp_lift_x
    // stack: y, sqrtOk, hash, isValid(v,r,s), r, s, retdest

    // If inputs are invalid or lifting fails, abort.
    SWAP3
    // stack: isValid(v,r,s), sqrtOk, hash, y, r, s, retdest
    AND
    // stack: isValid(v,r,s) & sqrtOk, hash, y, r, s, retdest
    %jumpi(ecrecover_valid_input)
    // stack: hash, y, r, s, retdest
    %pop4
    // stack: retdest
    %ecrecover_invalid_input

// ecrecover precompile.
// Assumption: Inputs are valid.
// Pseudo-code:
// let P = lift_x(r, recovery_id);
// let r_inv = r.inverse();
// let u1 = s * r_inv;
// let u2 = -hash * r_inv;
// return u1*P + u2*GENERATOR;
ecrecover_valid_input:
    JUMPDEST
    // stack: hash, y, r, s, retdest

    // Compute u1 = s * r^(-1)
    SWAP1
    // stack: y, hash, r, s, retdest
    DUP3
    // stack: r, y, hash, x, s, retdest (r=x)
    %inverse_secp_scalar
    // stack: r^(-1), y, hash, x, s, retdest
    DUP1
    // stack: r^(-1), r^(-1), y, hash, x, s, retdest
    SWAP5
    // stack: s, r^(-1), y, hash, x, r^(-1), retdest
    %mulmodn_secp_scalar
    // stack: u1, y, hash, x, r^(-1), retdest


    // Compute (X,Y) = u1 * (x,y)
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

// ecrecover precompile.
// Assumption: (X,Y) = u1 * P. Result is (X,Y) + u2*GENERATOR
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

    // Compute u2 = -hash * r^(-1)
    MOD
    // stack: hash%p, r^(-1), Y, X, retdest
    %secp_scalar
    // stack: p, hash%p, r^(-1), Y, X, retdest
    SUB
    // stack: -hash, r^(-1), Y, X, retdest
    %mulmodn_secp_scalar
    // stack: u2, Y, X, retdest

    // Compute u2 * GENERATOR and chain the call to `ec_mul` with a call to `ec_add` to compute PUBKEY = (X,Y) + u2 * GENERATOR,
    // and a call to `pubkey_to_addr` to get the final result `KECCAK256(PUBKEY)[-20:]`.
    PUSH pubkey_to_addr
    // stack: pubkey_to_addr, u2, Y, X, retdest
    SWAP3
    // stack: X, u2, Y, pubkey_to_addr, retdest
    PUSH ec_add_valid_points_secp
    // stack: ec_add_valid_points_secp, X, u2, Y, pubkey_to_addr, retdest
    SWAP1
    // stack: X, ec_add_valid_points_secp, u2, Y, pubkey_to_addr, retdest
    PUSH 0x79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798 // x-coordinate of generator
    // stack: Gx, X, ec_add_valid_points_secp, u2, Y, pubkey_to_addr, retdest
    SWAP1
    // stack: X, Gx, ec_add_valid_points_secp, u2, Y, pubkey_to_addr, retdest
    PUSH 0x483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8 // y-coordinate of generator
    // stack: Gy, X, Gx, ec_add_valid_points_secp, u2, Y, pubkey_to_addr, retdest
    SWAP1
    // stack: X, Gy, Gx, ec_add_valid_points_secp, u2, Y, pubkey_to_addr, retdest
    SWAP4
    // stack: u2, Gy, Gx, ec_add_valid_points_secp, X, Y, pubkey_to_addr, retdest
    SWAP2
    // stack: Gx, Gy, u2, ec_add_valid_points_secp, X, Y, pubkey_to_addr, retdest
    %jump(ec_mul_valid_point_secp)

// Take a public key (PKx, PKy) and return the associated address KECCAK256(PKx || PKy)[-20:].
pubkey_to_addr:
    JUMPDEST
    // stack: PKx, PKy, retdest
    PUSH 0
    // stack: 0, PKx, PKy, retdest
    MSTORE // TODO: switch to kernel memory (like `%mstore_current(@SEGMENT_KERNEL_GENERAL)`).
    // stack: PKy, retdest
    PUSH 0x20
    // stack: 0x20, PKy, retdest
    MSTORE
    // stack: retdest
    PUSH 0x40
    // stack: 0x40, retdest
    PUSH 0
    // stack: 0, 0x40, retdest
    KECCAK256
    // stack: hash, retdest
    PUSH 0xffffffffffffffffffffffffffffffffffffffff
    // stack: 2^160-1, hash, retdest
    AND
    // stack: address, retdest
    SWAP1
    // stack: retdest, address
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
