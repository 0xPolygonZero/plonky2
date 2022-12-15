// Computes the MSM `a*G + b*Q` used in ECDSA, see `ecdsa_msm_with_glv` in `ecrecover.asm`.
// Assumes wNAF expansion of `a0, a1, b0, b1` and precomputed tables for `G, Q` are in memory.
// Classic windowed MSM algorithm otherwise.
// Python code (without precomputed tables):
// def ecdsa_msm(nafs, points):
//     ans = O
//     n = len(nafs[0])
//     assert len(nafs) == len(points)
//     assert all(len(naf) == n for naf in nafs)
//     for i in range(n):
//         ss = [naf[-i-1] for naf in nafs]
//         assert all((x==0) or (x%2) for x in ss)
//         for x,point in zip(ss, points):
//             if x:
//                 if x > 15:
//                     ans -= (32-x)*point
//                 else:
//                     ans += x*point
//
//         if i < n-1:
//             ans *= 2
//     return ans
global ecdsa_msm:
    // stack: retdest
    PUSH 0 PUSH 0 PUSH 0
msm_loop:
    // stack: accx, accy, i, retdest
    DUP3 %mload_wnaf_a
    // stack: w, accx, accy, i, retdest
    DUP1 %jumpi(msm_loop_add_a_nonzero)
    POP
msm_loop_add_b:
    //stack: accx, accy, i, retdest
    DUP3 %mload_wnaf_b
    // stack: w, accx, accy, i, retdest
    DUP1 %jumpi(msm_loop_add_b_nonzero)
    POP
msm_loop_add_c:
    //stack: accx, accy, i, retdest
    DUP3 %mload_wnaf_c
    // stack: w, accx, accy, i, retdest
    DUP1 %jumpi(msm_loop_add_c_nonzero)
    POP
msm_loop_add_d:
    //stack: accx, accy, i, retdest
    DUP3 %mload_wnaf_d
    // stack: w, accx, accy, i, retdest
    DUP1 %jumpi(msm_loop_add_d_nonzero)
    POP
msm_loop_contd:
    %stack (accx, accy, i, retdest) -> (i, i, accx, accy, retdest)
    %eq_const(129) %jumpi(msm_end)
    %increment
    //stack: i+1, accx, accy, retdest
    %stack (i, accx, accy, retdest) -> (accx, accy, msm_loop, i, retdest)
    %jump(ec_double_secp)

msm_end:
    %stack (i, accx, accy, retdest) -> (retdest, accx, accy)
    JUMP

msm_loop_add_a_nonzero:
    %stack (w, accx, accy, i, retdest) -> (w, accx, accy, msm_loop_add_b, i, retdest)
    %mload_point_a
    // stack: px, py, accx, accy, msm_loop_add_b, i, retdest
    %jump(ec_add_valid_points_secp)

msm_loop_add_b_nonzero:
    %stack (w, accx, accy, i, retdest) -> (w, accx, accy, msm_loop_add_c, i, retdest)
    %mload_point_b
    // stack: px, py, accx, accy, msm_loop_add_c, i, retdest
    %jump(ec_add_valid_points_secp)

msm_loop_add_c_nonzero:
    %stack (w, accx, accy, i, retdest) -> (w, accx, accy, msm_loop_add_d, i, retdest)
    %mload_point_c
    // stack: px, py, accx, accy, msm_loop_add_d, i, retdest
    %jump(ec_add_valid_points_secp)

msm_loop_add_d_nonzero:
    %stack (w, accx, accy, i, retdest) -> (w, accx, accy, msm_loop_contd, i, retdest)
    %mload_point_d
    // stack: px, py, accx, accy, msm_loop_contd, i, retdest
    %jump(ec_add_valid_points_secp)

%macro mload_wnaf_a
    // stack: i
    %mload_kernel(@SEGMENT_KERNEL_WNAF_A)
%endmacro

%macro mload_wnaf_b
    // stack: i
    %mload_kernel(@SEGMENT_KERNEL_WNAF_B)
%endmacro

%macro mload_wnaf_c
    // stack: i
    %mload_kernel(@SEGMENT_KERNEL_WNAF_C)
%endmacro

%macro mload_wnaf_d
    // stack: i
    %mload_kernel(@SEGMENT_KERNEL_WNAF_D)
%endmacro

%macro mload_point_a
    // stack: w
    DUP1
    %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    //stack: Gy, w
    SWAP1 %decrement %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    //stack: Gx, Gy
%endmacro

%macro mload_point_b
    // stack: w
    DUP1
    %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    PUSH 1337 %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    %stack (a1neg, Gy, w) -> (@SECP_BASE, Gy, a1neg, a1neg, Gy, w)
    SUB SWAP1 ISZERO MUL SWAP2 MUL ADD
    SWAP1 %decrement %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    //stack: Gx, Gy
    PUSH @SECP_BASE
    SWAP1
    PUSH @SECP_GLV_BETA
    MULMOD
%endmacro

%macro mload_point_c
    // stack: w
    DUP1
    %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_Q)
    //stack: Qy, w
    SWAP1 %decrement %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_Q)
    //stack: Qx, Qy
%endmacro

%macro mload_point_d
    // stack: w
    DUP1
    %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_Q)
    //stack: Qy, w
    PUSH 1337 %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_Q)
    %stack (b1neg, Qy, w) -> (@SECP_BASE, Qy, b1neg, b1neg, Qy, w)
    SUB SWAP1 ISZERO MUL SWAP2 MUL ADD
    SWAP1 %decrement %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_Q)
    //stack: Qx, Qy
    PUSH @SECP_BASE
    SWAP1
    PUSH @SECP_GLV_BETA
    MULMOD
%endmacro
