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


before_precomputation:
    %stack (x, y, s, retdest) -> (x, y, after_precomputation, s, retdest)
    %jump(secp_precompute)

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

// Assumption: 2(x,y) = (x',y')
step_case_contd:
    // stack: x', y', s / 2, recursion_return, x, y, s, retdest
    %jump(ec_mul_valid_point_secp)

recursion_return:
    // stack: x', y', x, y, s, retdest
    SWAP4
    // stack: s, y', x, y, x', retdest
    PUSH 1
    // stack: 1, s, y', x, y, x', retdest
    AND
    // stack: s & 1, y', x, y, x', retdest
    SWAP1
    // stack: y', s & 1, x, y, x', retdest
    SWAP2
    // stack: x, s & 1, y', y, x', retdest
    SWAP3
    // stack: y, s & 1, y', x, x', retdest
    SWAP4
    // stack: x', s & 1, y', x, y, retdest
    SWAP1
    // stack: s & 1, x', y', x, y, retdest
    %jumpi(odd_scalar)
    // stack: x', y', x, y, retdest
    SWAP3
    // stack: y, y', x, x', retdest
    POP
    // stack: y', x, x', retdest
    SWAP1
    // stack: x, y', x', retdest
    POP
    // stack: y', x', retdest
    SWAP2
    // stack: retdest, x', y'
    JUMP

odd_scalar:
    // stack: x', y', x, y, retdest
    %jump(ec_add_valid_points_secp)
