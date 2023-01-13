// ecrecover precompile.
global ecrecover:
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

    // Compute u2 = -hash * r^(-1)
    %stack (u1, y, hash, x, rinv, retdest) -> (hash, @SECP_SCALAR, @SECP_SCALAR, rinv, @SECP_SCALAR, u1, x, y, pubkey_to_addr, retdest)
    MOD SWAP1 SUB MULMOD
    // stack: u2, u1, x, y, pubkey_to_addr, retdest
    %jump(ecdsa_msm_with_glv)

// Computes `a * G + b * Q` using GLV+wNAF, where `G` is the Secp256k1 generator and `Q` is a point on the curve.
// Pseudo-code:
// precompute_table(G) -- precomputation table for the base point, stored in memory
// let a0, a1 = glv_decompose(a)
// let wnaf_a0 = wnaf(a0) -- the wNAF values are stored in memory
// let wnaf_a1 = wnaf(a0)
// let b0, b1 = glv_decompose(b)
// let wnaf_b0 = wnaf(b0)
// let wnaf_b1 = wnaf(b0)
// precompute_table(Q) -- precomputation table for `Q`, stored in memory
// return msm_with_wnaf([wnaf_a0, wnaf_a1, wnaf_b0, wnaf_b1], [G, phi(G), Q, phi(Q)]) -- phi is the Secp endomorphism.
ecdsa_msm_with_glv:
    %stack (a, b, Qx, Qy, retdest) -> (a, ecdsa_after_glv_a, b, Qx, Qy, retdest)
    %jump(glv_decompose)
ecdsa_after_glv_a:
    %stack (a1neg, a0, a1, b, Qx, Qy, retdest) -> (b, ecdsa_after_glv_b, a1neg, a0, a1, Qx, Qy, retdest)
    %jump(glv_decompose)
ecdsa_after_glv_b:
    %stack (b1neg, b0, b1, a1neg, a0, a1, Qx, Qy, retdest) -> (a1neg, b1neg, Qx, Qy, ecdsa_after_precompute, a0, a1, b0, b1, retdest)
global wth:
    %jump(precompute_table_base_point)
ecdsa_after_precompute:
    // stack: a0, a1, b0, b1, retdest
    PUSH 0 PUSH 0 PUSH 129
    // stack: i, accx, accy, a0, a1, b0, b1, retdest
ecdsa_after_precompute_loop:
    %stack (i, accx, accy, a0, a1, b0, b1, retdest) -> (i, b1, i, accx, accy, a0, a1, b0, b1, retdest)
    SHR %and_const(1)
    %stack (bit_b1, i, accx, accy, a0, a1, b0, b1, retdest) -> (i, b0, bit_b1, i, accx, accy, a0, a1, b0, b1, retdest)
    SHR %and_const(1)
    %stack (bit_b0, bit_b1, i, accx, accy, a0, a1, b0, b1, retdest) -> (i, a1, bit_b0, bit_b1, i, accx, accy, a0, a1, b0, b1, retdest)
    SHR %and_const(1)
    %stack (bit_a1, bit_b0, bit_b1, i, accx, accy, a0, a1, b0, b1, retdest) -> (i, a0, bit_a1, bit_b0, bit_b1, i, accx, accy, a0, a1, b0, b1, retdest)
    SHR %and_const(1)
    %mul_const(2) ADD %mul_const(2) ADD %mul_const(2) ADD
    %stack (index, i, accx, accy, a0, a1, b0, b1, retdest) -> (index, index, i, accx, accy, a0, a1, b0, b1, retdest)
    %mul_const(2) %add_const(1)
    %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    SWAP1 %mul_const(2)
    %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
global wtf:
    %stack (Px, Py, i, accx, accy, a0, a1, b0, b1, retdest) -> (Px, Py, accx, accy, ecdsa_after_precompute_loop_contd, i, a0, a1, b0, b1, retdest)
    %jump(ec_add_valid_points_secp)
ecdsa_after_precompute_loop_contd:
    %stack (accx, accy, i, a0, a1, b0, b1, retdest) -> (i, accx, accy, ecdsa_after_precompute_loop_contd2, i, a0, a1, b0, b1, retdest)
    ISZERO %jumpi(ecdsa_after_precompute_loop_end)
    %jump(ec_double_secp)
ecdsa_after_precompute_loop_contd2:
    %stack (accx, accy, i, a0, a1, b0, b1, retdest) -> (i, accx, accy, a0, a1, b0, b1, retdest)
    %decrement %jump(ecdsa_after_precompute_loop)
ecdsa_after_precompute_loop_end:
    %stack (accx, accy, ecdsa_after_precompute_loop_contd2, i, a0, a1, b0, b1, retdest) -> (retdest, accx, accy)
    JUMP

/* ecdsa_after_a0:
    // stack: a1, b, Qx, Qy, retdest
    PUSH ecdsa_after_a1 SWAP1 PUSH @SEGMENT_KERNEL_WNAF_B %jump(wnaf)
ecdsa_after_a1:
    // stack: b, Qx, Qy, retdest
    %stack (b, Qx, Qy, retdest) -> (b, ecdsa_after_glv_b, Qx, Qy, retdest)
    %jump(glv_decompose)
ecdsa_after_glv_b:
    // stack: b1neg, b0, b1, Qx, Qy, retdest
    // Store b1neg at this (otherwise unused) location. Will be used later in the MSM.
    %mstore_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_Q, 1337)
    // stack: b0, b1, Qx, Qy, retdest
    PUSH ecdsa_after_b0 SWAP1 PUSH @SEGMENT_KERNEL_WNAF_C %jump(wnaf)
ecdsa_after_b0:
    // stack: d, Qx, Qy, retdest
    PUSH ecdsa_after_b1 SWAP1 PUSH @SEGMENT_KERNEL_WNAF_D %jump(wnaf)
ecdsa_after_b1:
    %stack (Qx, Qy, retdest) -> (Qx, Qy, ecdsa_after_precompute, retdest)
    %jump(precompute_table)
ecdsa_after_precompute:
    // stack: retdest
    %jump(ecdsa_msm)
    %stack (accx, accy, retdest) -> (retdest, accx, accy)
    JUMP */


// Take a public key (PKx, PKy) and return the associated address KECCAK256(PKx || PKy)[-20:].
pubkey_to_addr:
    // stack: PKx, PKy, retdest
    PUSH 0
    // stack: 0, PKx, PKy, retdest
    MSTORE // TODO: switch to kernel memory (like `%mstore_kernel(@SEGMENT_KERNEL_GENERAL)`).
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
    PUSH @SECP_SCALAR
%endmacro

// Return u256::MAX which is used to indicate the input was invalid.
%macro ecrecover_invalid_input
    // stack: retdest
    PUSH @U256_MAX
    // stack: u256::MAX, retdest
    SWAP1
    // stack: retdest, u256::MAX
    JUMP
%endmacro
