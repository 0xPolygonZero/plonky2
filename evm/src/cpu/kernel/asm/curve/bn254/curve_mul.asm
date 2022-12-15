// BN254 elliptic curve scalar multiplication.
// Recursive implementation, same algorithm as in `exp.asm`.
global ec_mul:
    // stack: x, y, s, retdest
    DUP2
    // stack: y, x, y, s, retdest
    DUP2
    // stack: x, y, x, y, s, retdest
    %ec_isidentity
    // stack: (x,y)==(0,0), x, y, s, retdest
    %jumpi(ret_zero_ec_mul)
    // stack: x, y, s, retdest
    DUP2
    // stack: y, x, y, s, retdest
    DUP2
    // stack: x, y, x, y, s, retdest
    %ec_check
    // stack: isValid(x, y), x, y, s, retdest
    %jumpi(ec_mul_valid_point)
    // stack: x, y, s, retdest
    %pop3
    %ec_invalid_input

// Same algorithm as in `exp.asm`
ec_mul_valid_point:
    %stack (x, y, s, retdest) -> (s, ec_mul_after_glv, x, y, bn_msm, ec_mul_end, retdest)
    %jump(bn_glv_decompose)
global ec_mul_after_glv:
    // stack: bneg, a, b, x, y, bn_msm, ec_mul_after_glv_precompute_and_msm, retdest
    // Store bneg at this (otherwise unused) location. Will be used later in the MSM.
    %mstore_kernel(@SEGMENT_KERNEL_BN_TABLE_Q, 1337)
    // stack: a, b, x, y, bn_msm, ec_mul_after_glv_precompute_and_msm, retdest
    PUSH ec_mul_after_a SWAP1 PUSH @SEGMENT_KERNEL_BN_WNAF_A PUSH @BN_SCALAR %jump(wnaf)
global ec_mul_after_a:
    // stack: b, x, y, bn_msm, ec_mul_after_glv_precompute_and_msm, retdest
    PUSH ec_mul_after_b SWAP1 PUSH @SEGMENT_KERNEL_BN_WNAF_B PUSH @BN_SCALAR %jump(wnaf)
global ec_mul_after_b:
    // stack: x, y, bn_msm, ec_mul_end, retdest
    %jump(bn_precompute_table)
global ec_mul_end:
    %stack (Ax, Ay, retdest) -> (retdest, Ax, Ay)
    JUMP