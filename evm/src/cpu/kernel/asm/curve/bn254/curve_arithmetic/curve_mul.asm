// BN254 elliptic curve scalar multiplication.
// Uses GLV, wNAF with w=5, and a MSM algorithm.
global bn_mul:
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
    %bn_check
    // stack: isValid(x, y), x, y, s, retdest
    %jumpi(bn_mul_valid_point)
    // stack: x, y, s, retdest
    %pop3
    %bn_invalid_input

bn_mul_valid_point:
    %stack (x, y, s, retdest) -> (s, bn_mul_after_glv, x, y, bn_msm, bn_mul_end, retdest)
    %jump(bn_glv_decompose)
bn_mul_after_glv:
    // stack: bneg, a, b, x, y, bn_msm, bn_mul_end, retdest
    // Store bneg at this (otherwise unused) location. Will be used later in the MSM.
    %mstore_current(@SEGMENT_BN_TABLE_Q, @BN_BNEG_LOC)
    // stack: a, b, x, y, bn_msm, bn_mul_end, retdest
    PUSH bn_mul_after_a SWAP1 PUSH @SEGMENT_BN_WNAF_A PUSH @BN_SCALAR %jump(wnaf)
bn_mul_after_a:
    // stack: b, x, y, bn_msm, bn_mul_end, retdest
    PUSH bn_mul_after_b SWAP1 PUSH @SEGMENT_BN_WNAF_B PUSH @BN_SCALAR %jump(wnaf)
bn_mul_after_b:
    // stack: x, y, bn_msm, bn_mul_end, retdest
    %jump(bn_precompute_table)
bn_mul_end:
    %stack (Ax, Ay, retdest) -> (retdest, Ax, Ay)
    JUMP
