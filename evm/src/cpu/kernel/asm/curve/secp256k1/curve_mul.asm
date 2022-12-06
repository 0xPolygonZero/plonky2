// Same algorithm as in `exp.asm`
global ec_mul_valid_point_secp:
    // stack: x, y, s, retdest
    %stack (x,y) -> (x,y,x,y)
    %ec_isidentity
    // stack: (x,y)==(0,0), x, y, s, retdest
    %jumpi(ret_zero_ec_mul)
    DUP3
    // stack: s, x, y, s, retdest
    %jumpi(before_precomputation)
    // stack: x, y, s, retdest
    %jump(ret_zero_ec_mul)

before_precomputation:
    %stack (x, y, s, retdest) -> (x, y, after_precomputation, s, retdest)
    %jump(secp_precompute)

// Write `(xi, yi) = i * (x, y)` in memory at locations `2i, 2i+1` for `i = 0,...,15`.
secp_precompute:
    // stack: x, y, retdest
    DUP2 DUP2
    %stack (accx, accy, x, y, retdest) -> (2, accx, 3, accy, accx, accy, x, y, retdest)
    %mstore_kernel_general %mstore_kernel_general
    %stack (accx, accy, x, y, retdest) -> (accx, accy, x, y, secp_precompute_1, x, y, retdest)
    %jump(ec_add_valid_points_secp)
secp_precompute_1:
    %stack (accx, accy, x, y, retdest) -> (4, accx, 5, accy, accx, accy, x, y, retdest)
    %mstore_kernel_general %mstore_kernel_general
    %stack (accx, accy, x, y, retdest) -> (accx, accy, x, y, secp_precompute_2, x, y, retdest)
    %jump(ec_add_valid_points_secp)
secp_precompute_2:
    %stack (accx, accy, x, y, retdest) -> (6, accx, 7, accy, accx, accy, x, y, retdest)
    %mstore_kernel_general %mstore_kernel_general
    %stack (accx, accy, x, y, retdest) -> (accx, accy, x, y, secp_precompute_3, x, y, retdest)
    %jump(ec_add_valid_points_secp)
secp_precompute_3:
    %stack (accx, accy, x, y, retdest) -> (8, accx, 9, accy, accx, accy, x, y, retdest)
    %mstore_kernel_general %mstore_kernel_general
    %stack (accx, accy, x, y, retdest) -> (accx, accy, x, y, secp_precompute_4, x, y, retdest)
    %jump(ec_add_valid_points_secp)
secp_precompute_4:
    %stack (accx, accy, x, y, retdest) -> (10, accx, 11, accy, accx, accy, x, y, retdest)
    %mstore_kernel_general %mstore_kernel_general
    %stack (accx, accy, x, y, retdest) -> (accx, accy, x, y, secp_precompute_5, x, y, retdest)
    %jump(ec_add_valid_points_secp)
secp_precompute_5:
    %stack (accx, accy, x, y, retdest) -> (12, accx, 13, accy, accx, accy, x, y, retdest)
    %mstore_kernel_general %mstore_kernel_general
    %stack (accx, accy, x, y, retdest) -> (accx, accy, x, y, secp_precompute_6, x, y, retdest)
    %jump(ec_add_valid_points_secp)
secp_precompute_6:
    %stack (accx, accy, x, y, retdest) -> (14, accx, 15, accy, accx, accy, x, y, retdest)
    %mstore_kernel_general %mstore_kernel_general
    %stack (accx, accy, x, y, retdest) -> (accx, accy, x, y, secp_precompute_7, x, y, retdest)
    %jump(ec_add_valid_points_secp)
secp_precompute_7:
    %stack (accx, accy, x, y, retdest) -> (16, accx, 17, accy, accx, accy, x, y, retdest)
    %mstore_kernel_general %mstore_kernel_general
    %stack (accx, accy, x, y, retdest) -> (accx, accy, x, y, secp_precompute_8, x, y, retdest)
    %jump(ec_add_valid_points_secp)
secp_precompute_8:
    %stack (accx, accy, x, y, retdest) -> (18, accx, 19, accy, accx, accy, x, y, retdest)
    %mstore_kernel_general %mstore_kernel_general
    %stack (accx, accy, x, y, retdest) -> (accx, accy, x, y, secp_precompute_9, x, y, retdest)
    %jump(ec_add_valid_points_secp)
secp_precompute_9:
    %stack (accx, accy, x, y, retdest) -> (20, accx, 21, accy, accx, accy, x, y, retdest)
    %mstore_kernel_general %mstore_kernel_general
    %stack (accx, accy, x, y, retdest) -> (accx, accy, x, y, secp_precompute_a, x, y, retdest)
    %jump(ec_add_valid_points_secp)
secp_precompute_a:
    %stack (accx, accy, x, y, retdest) -> (22, accx, 23, accy, accx, accy, x, y, retdest)
    %mstore_kernel_general %mstore_kernel_general
    %stack (accx, accy, x, y, retdest) -> (accx, accy, x, y, secp_precompute_b, x, y, retdest)
    %jump(ec_add_valid_points_secp)
secp_precompute_b:
    %stack (accx, accy, x, y, retdest) -> (24, accx, 25, accy, accx, accy, x, y, retdest)
    %mstore_kernel_general %mstore_kernel_general
    %stack (accx, accy, x, y, retdest) -> (accx, accy, x, y, secp_precompute_c, x, y, retdest)
    %jump(ec_add_valid_points_secp)
secp_precompute_c:
    %stack (accx, accy, x, y, retdest) -> (26, accx, 27, accy, accx, accy, x, y, retdest)
    %mstore_kernel_general %mstore_kernel_general
    %stack (accx, accy, x, y, retdest) -> (accx, accy, x, y, secp_precompute_d, x, y, retdest)
    %jump(ec_add_valid_points_secp)
secp_precompute_d:
    %stack (accx, accy, x, y, retdest) -> (28, accx, 29, accy, accx, accy, x, y, retdest)
    %mstore_kernel_general %mstore_kernel_general
    %stack (accx, accy, x, y, retdest) -> (accx, accy, x, y, secp_precompute_e, x, y, retdest)
    %jump(ec_add_valid_points_secp)
secp_precompute_e:
    %stack (accx, accy, x, y, retdest) -> (30, accx, 31, accy, accx, accy, x, y, retdest)
    %mstore_kernel_general %mstore_kernel_general
    %pop4 JUMP


// Windowed scalar multiplication with a 4-bit window.
after_precomputation:
    // stack: s, retdest
    PUSH 0 PUSH 0 PUSH 0
mul_loop:
    // stack: i, accx, accy, s, retdest
    %stack (i, accx, accy, s, retdest) -> (252, s, accx, accy, mul_loop_contd, mul_loop_contd_bis, s, i, retdest)
    SHR
    // stack: s>>252, accx, accy, mul_loop_contd, mul_loop_contd_bis, s, i, retdest
    %mul_const(2)
    // stack: index, accx, accy, mul_loop_contd, mul_loop_contd_bis, s, i, retdest
    DUP1 %increment
    // stack: index+1, index, accx, accy, mul_loop_contd, mul_loop_contd_bis, s, i, retdest
    %mload_kernel_general SWAP1 %mload_kernel_general
    // stack: x, y, accx, accy, mul_loop_contd, mul_loop_contd_bis, s, i, retdest
    %jump(ec_add_valid_points_secp)
mul_loop_contd:
    DUP5
    %eq_const(63) %jumpi(mul_end)
    %jump(repeated_double)
mul_loop_contd_bis:
    // stack: accx, accy, s, i, retdest
    SWAP2
    // stack: s, accy, accx, i, retdest
    %shl_const(4)
    // stack: news, accy, accx, i, retdest
    %stack (s, accy, accx, i, retdest) -> (i, accx, accy, s, retdest)
    %increment %jump(mul_loop)
mul_end:
    %stack (accx, accy, mul_loop_contd_bis, s, i, retdest) -> (retdest, accx, accy)
    JUMP


// Computes `16 * (x, y)` by doubling 4 times.
repeated_double:
    // stack: x, y, retdest
    %stack (x, y, retdest) -> (x, y, repeated_double_1, retdest)
    %jump(ec_double_secp)
repeated_double_1:
    %stack (x, y, retdest) -> (x, y, repeated_double_2, retdest)
    %jump(ec_double_secp)
repeated_double_2:
    %stack (x, y, retdest) -> (x, y, repeated_double_3, retdest)
    %jump(ec_double_secp)
repeated_double_3:
    %stack (x, y, retdest) -> (x, y, repeated_double_4, retdest)
    %jump(ec_double_secp)
repeated_double_4:
    %stack (x, y, retdest) -> (retdest, x, y)
    JUMP

wnaf:
    // stack: s, retdest
    PUSH 0
    SWAP1
    // stack: s, o, retdest
wnaf_loop:
    DUP1 ISZERO %jumpi(wnaf_end)
    // stack: s, o, retdest
    DUP1

wnaf_end:
    // stack: s, o, retdest
    %pop2 JUMP

global trailing_zeros0:
    PUSH 0 %jump(yo)
global trailing_zeros1:
    PUSH 0 %jump(yo)
global trailing_zeros2:
    PUSH 1 %jump(yo)

yo:
