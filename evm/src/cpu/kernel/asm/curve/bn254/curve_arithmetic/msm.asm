// Computes the multiplication `a*G` using a standard MSM with the GLV decomposition of `a`.
// see there for a detailed description.
global bn_msm:
    // stack: retdest
    PUSH 0 PUSH 0 PUSH 0
global bn_msm_loop:
    // stack: accx, accy, i, retdest
    DUP3 %bn_mload_wnaf_a
    // stack: w, accx, accy, i, retdest
    DUP1 %jumpi(bn_msm_loop_add_a_nonzero)
    POP
msm_loop_add_b:
    //stack: accx, accy, i, retdest
    DUP3 %bn_mload_wnaf_b
    // stack: w, accx, accy, i, retdest
    DUP1 %jumpi(bn_msm_loop_add_b_nonzero)
    POP
msm_loop_contd:
    %stack (accx, accy, i, retdest) -> (i, i, accx, accy, retdest)
    // TODO: the GLV scalars for the BN curve are 127-bit, so could use 127 here. But this would require modifying `wnaf.asm`. Not sure it's worth it...
    %eq_const(129) %jumpi(msm_end)
    %increment
    //stack: i+1, accx, accy, retdest
    %stack (i, accx, accy, retdest) -> (accx, accy, bn_msm_loop, i, retdest)
    %jump(bn_double)

msm_end:
    %stack (i, accx, accy, retdest) -> (retdest, accx, accy)
    JUMP

bn_msm_loop_add_a_nonzero:
    %stack (w, accx, accy, i, retdest) -> (w, accx, accy, msm_loop_add_b, i, retdest)
    %bn_mload_point_a
    // stack: px, py, accx, accy, msm_loop_add_b, i, retdest
    %jump(bn_add_valid_points)

bn_msm_loop_add_b_nonzero:
    %stack (w, accx, accy, i, retdest) -> (w, accx, accy, msm_loop_contd, i, retdest)
    %bn_mload_point_b
    // stack: px, py, accx, accy, msm_loop_contd, i, retdest
    %jump(bn_add_valid_points)

%macro bn_mload_wnaf_a
    // stack: i
    %mload_current(@SEGMENT_BN_WNAF_A)
%endmacro

%macro bn_mload_wnaf_b
    // stack: i
    %mload_current(@SEGMENT_BN_WNAF_B)
%endmacro

%macro bn_mload_point_a
    // stack: w
    DUP1
    %mload_current(@SEGMENT_BN_TABLE_Q)
    //stack: Gy, w
    SWAP1 %decrement %mload_current(@SEGMENT_BN_TABLE_Q)
    //stack: Gx, Gy
%endmacro

%macro bn_mload_point_b
    // stack: w
    DUP1
    %mload_current(@SEGMENT_BN_TABLE_Q)
    PUSH @BN_BNEG_LOC %mload_current(@SEGMENT_BN_TABLE_Q)
    %stack (bneg, Gy, w) -> (@BN_BASE, Gy, bneg, bneg, Gy, w)
    SUB SWAP1 ISZERO MUL SWAP2 MUL ADD
    SWAP1 %decrement %mload_current(@SEGMENT_BN_TABLE_Q)
    //stack: Gx, Gy
    PUSH @BN_GLV_BETA
    MULFP254
%endmacro
