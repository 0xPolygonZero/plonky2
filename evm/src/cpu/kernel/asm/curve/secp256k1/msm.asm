global ecdsa_msm:
    // stack: retdest
    PUSH 0 PUSH 0 PUSH 0
global msm_loop:
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

global msm_loop_add_a_nonzero:
    %stack (w, accx, accy, i, retdest) -> (w, accx, accy, msm_loop_add_b, i, retdest)
    %mload_point_a
    // stack: px, py, accx, accy, msm_loop_add_b, i, retdest
    %jump(ec_add_valid_points_secp)

global msm_loop_add_b_nonzero:
    %stack (w, accx, accy, i, retdest) -> (w, accx, accy, msm_loop_add_c, i, retdest)
    %mload_point_b
    // stack: px, py, accx, accy, msm_loop_add_c, i, retdest
    %jump(ec_add_valid_points_secp)

global msm_loop_add_c_nonzero:
    %stack (w, accx, accy, i, retdest) -> (w, accx, accy, msm_loop_add_d, i, retdest)
    %mload_point_c
    // stack: px, py, accx, accy, msm_loop_add_d, i, retdest
    %jump(ec_add_valid_points_secp)

global msm_loop_add_d_nonzero:
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
    //stack: Gy, w
    SWAP1 %decrement %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_G)
    //stack: Gx, Gy
    PUSH 115792089237316195423570985008687907853269984665640564039457584007908834671663
    SWAP1
    PUSH 0x7ae96a2b657c07106e64479eac3434e99cf0497512f58995c1396c28719501ee
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
    SWAP1 %decrement %mload_kernel(@SEGMENT_KERNEL_ECDSA_TABLE_Q)
    //stack: Qx, Qy
    PUSH 115792089237316195423570985008687907853269984665640564039457584007908834671663
    SWAP1
    PUSH 0x7ae96a2b657c07106e64479eac3434e99cf0497512f58995c1396c28719501ee
    MULMOD
%endmacro
