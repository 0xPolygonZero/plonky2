global ecdsa_msm:
    PUSH 0 PUSH 0 PUSH 0
msm_loop:
    // stack: accx, accy, i, retdest
    DUP3 %mload_wnaf_a
    %stack (w, accx, accy, i, retdest) -> (w, accx, accy, msm_loop_add_b, i, retdest)
    %mload_point_a
    // stack: px, py, accx, accy, msm_loop_add_b, i, retdest
    %jump(ec_add_valid_points_secp)
msm_loop_add_b:
    //stack: accx, accy, i, retdest
    DUP3 %mload_wnaf_b
    %stack (w, accx, accy, i, retdest) -> (w, accx, accy, msm_loop_add_c, i, retdest)
    %mload_point_b
    // stack: px, py, accx, accy, msm_loop_add_c, i, retdest
    %jump(ec_add_valid_points_secp)
msm_loop_add_c:
    //stack: accx, accy, i, retdest
    DUP3 %mload_wnaf_c
    %stack (w, accx, accy, i, retdest) -> (w, accx, accy, msm_loop_add_d, i, retdest)
    %mload_point_c
    // stack: px, py, accx, accy, msm_loop_add_d, i, retdest
    %jump(ec_add_valid_points_secp)
msm_loop_add_d:
    //stack: accx, accy, i, retdest
    DUP3 %mload_wnaf_d
    %stack (w, accx, accy, i, retdest) -> (w, accx, accy, msm_loop_contd, i, retdest)
    %mload_point_d
    // stack: px, py, accx, accy, msm_loop_contd, i, retdest
    %jump(ec_add_valid_points_secp)
msm_loop_contd:
    %stack (accx, accy, i, retdest) -> (i, accx, accy, retdest)
    DUP1
    %eq_const(127) %jumpi(msm_end)
    %increment
    //stack: i+1, accx, accy, retdest
    %stack (i, accx, accy, retdest) -> (accx, accy, msm_loop, i, retdest)
    %jump(ec_double_secp)

msm_end:
    %stack (i, accx, accy, retdest) -> (retdest, accx, accy)
    JUMP


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
    %mload_kernel(@SEGMENT_KERNEL_WNAF_A)
%endmacro
