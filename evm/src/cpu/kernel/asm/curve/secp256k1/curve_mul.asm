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
    DUP2 DUP2 PUSH 2
secp_precompute_loop:
    // stack: i, accx, accy, x, y, retdest
    DUP1 %increment DUP1 %increment
    %stack (ssi, si, i, accx, accy, x, y, retdest) -> (i, accx, si, accy, accx, accy, x, y, ssi, retdest)
    %mstore_kernel_general %mstore_kernel_general
    %stack (accx, accy, x, y, ssi, retdest) -> (accx, accy, x, y, secp_precompute_loop_contd, x, y, ssi, retdest)
    %jump(ec_add_valid_points_secp)
secp_precompute_loop_contd:
    %stack (accx, accy, x, y, ssi, retdest) -> (ssi, accx, accy, x, y, retdest)
    DUP1 %lt_const(32)
    // stack: ssi < 32, ssi, accx, accy, x, y, retdest
    %jumpi(secp_precompute_loop)
    // stack: ssi, accx, accy, x, y, retdest
    %pop5 JUMP


// Windowed scalar multiplication with a 4-bit window.
after_precomputation:
    // stack: s, retdest
    PUSH 0 PUSH 0 PUSH 0
global mul_loop:
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
global mul_loop_contd:
    //%stack (accx, accy, mul_loop_contd_bis, s, i, retdest) -> (i, accx, accy, mul_loop_contd_bis, s, i, retdest)
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
    PUSH 0
repeated_double_loop:
    // stack: i, x, y, retdest
    DUP1 %eq_const(4) %jumpi(repeated_double_end)
    %stack (i, x, y, retdest) -> (x, y, repeated_double_loop_contd, i, retdest)
    %jump(ec_double_secp)
repeated_double_loop_contd:
    %stack (x2, y2, i, retdest) -> (i, x2, y2, retdest)
    %increment %jump(repeated_double_loop)
repeated_double_end:
    %stack (i, x, y, retdest) -> (retdest, x, y)
    JUMP
