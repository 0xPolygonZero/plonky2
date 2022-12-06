/// def miller(P, Q):
///     miller_init()
///     miller_loop()
///
/// def miller_init():
///     out = 1
///     O = P
///     times = 62
///
/// def miller_loop():
///     while times:
///         0xnm = load(miller_data)
///         while 0xnm > 0x10:
///             miller_one()
///         while 0xnm:
///             miller_zero()
///         times -= 1
///
/// def miller_one():
///     0xnm -= 0x10
///     mul_tangent()
///     mul_cord()
///
/// def miller_zero():
///     0xnm -= 1
///     mul_tangent()

/// Note: miller_data was defined by
/// (1) taking the binary expansion of the BN254 prime p
/// (2) popping the head and appending a 0:
///     exp = bin(p)[1:-1] + [0]
/// (3) counting the lengths of runs of 1s then 0s in exp, e.g.
///     exp = 1100010011110 => EXP = [(2,3), (1,2), (4,1)]
/// (4) encoding each pair (n,m) as 0xnm:
///     miller_data = [(0x10)n + m for (n,m) in EXP]

global miller_init:
    // stack:         ptr, out, retdest
    PUSH 1
    // stack:      1, ptr, out, retdest
    DUP3
    // stack: out, 1, ptr, out, retdest
    %mstore_kernel_general
    // stack:         ptr, out, retdest
    %load_fp6
    // stack:        P, Q, out, retdest
    DUP2  DUP2
    // stack:     O, P, Q, out, retdest
    PUSH 62
    // stack: 62, O, P, Q, out, retdest
miller_loop:
    // stack:          times  , O, P, Q, out, retdest
    DUP1  ISZERO
    // stack:  break?, times  , O, P, Q, out, retdest
    %jumpi(miller_end)
    // stack:          times  , O, P, Q, out, retdest
    %sub_const(1)
    // stack:          times-1, O, P, Q, out, retdest
    DUP1
    // stack: times-1, times-1, O, P, Q, out, retdest
    %mload_kernel_code(miller_data)
    // stack:    0xnm, times-1, O, P, Q, out, retdest
    %jump(miller_one)
miller_end:
    // stack: times, O, P, Q, out, retdest
    %pop3  %pop3  %pop3
    // stack:                 out, retdest
    %jump(post_mllr)


miller_one:
    // stack:               0xnm, times, O, P, Q, out, retdest
    DUP1  %gt_const(0x10) 
    // stack:        skip?, 0xnm, times, O, P, Q, out, retdest
    %jumpi(miller_zero)
    // stack:               0xnm, times, O, P, Q, out, retdest
    %sub_const(0x10)
    // stack:           0x{n-1}m, times, O, P, Q, out, retdest
    PUSH mul_cord
    // stack: mul_cord, 0x{n-1}m, times, O, P, Q, out, retdest
    %jump(mul_tangent)

miller_zero:
    // stack:              m  , times, O, P, Q, out, retdest
    DUP1  ISZERO
    // stack:       skip?, m  , times, O, P, Q, out, retdest
    %jumpi(miller_loop)
    // stack:              m  , times, O, P, Q, out, retdest
    %sub_const(1)
    // stack:              m-1, times, O, P, Q, out, retdest
    PUSH miller_zero
    // stack: miller_zero, m-1, times, O, P, Q, out, retdest
    %jump(mul_tangent)


/// def mul_tangent()
///     out = square_fp12(out)
///     line = tangent(O, Q)
///     out = mul_fp12_sparse(out, line)
///     O += O

mul_tangent:
    // stack:                                         retdest, 0xnm, times, O, P, Q, out
    PUSH mul_tangent_2  PUSH mul_tangent_1
    // stack:           mul_tangent_1, mul_tangent_2, retdest, 0xnm, times, O, P, Q, out
    DUP13  DUP1
    // stack: out, out, mul_tangent_1, mul_tangent_2, retdest, 0xnm, times, O, P, Q, out
    %jump(square_fp12)
mul_tangent_1:
    // stack:           out, mul_tangent_2, retdest, 0xnm, times, O, P, Q, out
    DUP12  DUP12  DUP12  DUP12
    // stack:        Q, out, mul_tangent_2, retdest, 0xnm, times, O, P, Q, out
    DUP10  DUP10
    // stack:     O, Q, out, mul_tangent_2, retdest, 0xnm, times, O, P, Q, out
    %store_tangent
    // stack:           out, mul_tangent_2, retdest, 0xnm, times, O, P, Q, out  {100: line}
    PUSH 100  DUP2
    // stack: out, 100, out, mul_tangent_2, retdest, 0xnm, times, O, P, Q, out  {100: line}
    %jump(mul_fp12_sparse)
mul_tangent_2:
    // stack:             out, retdest, 0xnm, times,   O, P, Q, out  {100: line}
    POP  PUSH after_double
    // stack:    after_double, retdest, 0xnm, times,   O, P, Q, out  {100: line}
    DUP5  DUP5
    // stack: O, after_double, retdest, 0xnm, times,   O, P, Q, out  {100: line}
    %jump(ec_double)
after_double:
    // stack:             2*O, retdest, 0xnm, times,   O, P, Q, out  {100: line}
    SWAP5  POP  SWAP5  POP
    // stack:                  retdest, 0xnm, times, 2*O, P, Q, out  {100: line}
    JUMP


/// def mul_cord()
///     line = cord(P, O, Q)
///     out = mul_fp12_sparse(out, line)
///     O += P

mul_cord:
    // stack:                            0xnm, times, O, P, Q, out
    PUSH mul_cord_1
    // stack:                mul_cord_1, 0xnm, times, O, P, Q, out
    DUP11  DUP11  DUP11  DUP11
    // stack:             Q, mul_cord_1, 0xnm, times, O, P, Q, out
    DUP9  DUP9
    // stack:          O, Q, mul_cord_1, 0xnm, times, O, P, Q, out
    DUP13  DUP13
    // stack:       P, O, Q, mul_cord_1, 0xnm, times, O, P, Q, out
    %store_cord 
    // stack:                mul_cord_1, 0xnm, times, O, P, Q, out  {100: line}
    DUP12
    // stack:           out, mul_cord_1, 0xnm, times, O, P, Q, out  {100: line}
    PUSH 100
    // stack:      100, out, mul_cord_1, 0xnm, times, O, P, Q, out  {100: line}
    DUP2
    // stack: out, 100, out, mul_cord_1, 0xnm, times, O, P, Q, out  {100: line}
    %jump(mul_fp12_sparse)
mul_cord_1:
    // stack:                   0xnm, times, O  , P, Q, out
    PUSH after_add
    // stack:        after_add, 0xnm, times, O  , P, Q, out
    DUP7  DUP7  DUP7  DUP7
    // stack: O , P, after_add, 0xnm, times, O  , P, Q, out
    %jump(ec_add_valid_points)
after_add:
    // stack:            O + P, 0xnm, times, O  , P, Q, out
    SWAP4  POP  SWAP4  POP
    // stack:                   0xnm, times, O+P, P, Q, out
    %jump(miller_one)


/// def store_cord(p1x, p1y, p2x, p2y, qx, qy):
///     return sparse_store(
///         p1y*p2x - p2y*p1x, 
///         (p2y - p1y) * qx, 
///         (p1x - p2x) * qy,
///     )

%macro store_cord
    // stack:                    p1x , p1y, p2x , p2y, qx, qx_, qy, qy_
    DUP1  DUP5  MULFP254
    // stack:           p2y*p1x, p1x , p1y, p2x , p2y, qx, qx_, qy, qy_
    DUP3  DUP5  MULFP254
    // stack: p1y*p2x , p2y*p1x, p1x , p1y, p2x , p2y, qx, qx_, qy, qy_
    SUBFP254
    // stack: p1y*p2x - p2y*p1x, p1x , p1y, p2x , p2y, qx, qx_, qy, qy_
    PUSH 100  %mstore_kernel_general
    // stack:                    p1x , p1y, p2x , p2y, qx, qx_, qy, qy_
    SWAP3
    // stack:                    p2y , p1y, p2x , p1x, qx, qx_, qy, qy_
    SUBFP254
    // stack:                    p2y - p1y, p2x , p1x, qx, qx_, qy, qy_
    SWAP2
    // stack:                    p1x , p2x, p2y - p1y, qx, qx_, qy, qy_
    SUBFP254
    // stack:                    p1x - p2x, p2y - p1y, qx, qx_, qy, qy_
    SWAP4
    // stack:                    qy, p2y - p1y, qx, qx_, p1x - p2x, qy_
    DUP5  MULFP254
    // stack:         (p1x - p2x)qy, p2y - p1y, qx, qx_, p1x - p2x, qy_
    PUSH 108  %mstore_kernel_general
    // stack:                        p2y - p1y, qx, qx_, p1x - p2x, qy_
    SWAP1
    // stack:                        qx, p2y - p1y, qx_, p1x - p2x, qy_
    DUP2  MULFP254
    // stack:             (p2y - p1y)qx, p2y - p1y, qx_, p1x - p2x, qy_
    PUSH 102  %mstore_kernel_general
    // stack:                            p2y - p1y, qx_, p1x - p2x, qy_
    MULFP254
    // stack:                            (p2y - p1y)qx_, p1x - p2x, qy_
    PUSH 103  %mstore_kernel_general
    // stack:                                            p1x - p2x, qy_
    MULFP254
    // stack:                                            (p1x - p2x)qy_
    PUSH 109  %mstore_kernel_general
%endmacro


/// def store_tangent(px, py, qx, qy):
///     return sparse_store(
///         py**2 - 9, 
///         (-3px**2) * qx, 
///         (2py)     * qy,
///     )

%macro store_tangent
    // stack:                px, py, qx, qx_, qy, qy_
    PUSH 9
    // stack:             9, px, py, qx, qx_, qy, qy_
    DUP3
    // stack:        py , 9, px, py, qx, qx_, qy, qy_
    DUP1  MULFP254
    // stack:     py**2 , 9, px, py, qx, qx_, qy, qy_
    SUBFP254
    // stack:     py**2 - 9, px, py, qx, qx_, qy, qy_
    PUSH 100  %mstore_kernel_general
    // stack:                px, py, qx, qx_, qy, qy_
    DUP1  MULFP254
    // stack:             px**2, py, qx, qx_, qy, qy_
    PUSH 3  MULFP254
    // stack:           3*px**2, py, qx, qx_, qy, qy_
    PUSH 0  SUBFP254
    // stack:          -3*px**2, py, qx, qx_, qy, qy_
    SWAP2
    // stack:           qx, py, -3px**2, qx_, qy, qy_
    DUP3  MULFP254
    // stack: (-3*px**2)qx, py, -3px**2, qx_, qy, qy_ 
    PUSH 102  %mstore_kernel_general
    // stack:               py, -3px**2, qx_, qy, qy_ 
    PUSH 2  MULFP254
    // stack:              2py, -3px**2, qx_, qy, qy_ 
    SWAP3 
    // stack:              qy, -3px**2, qx_, 2py, qy_ 
    DUP4  MULFP254
    // stack:         (2py)qy, -3px**2, qx_, 2py, qy_ 
    PUSH 108  %mstore_kernel_general
    // stack:                  -3px**2, qx_, 2py, qy_ 
    MULFP254
    // stack:                  (-3px**2)qx_, 2py, qy_ 
    PUSH 103  %mstore_kernel_general
    // stack:                                2py, qy_ 
    MULFP254
    // stack:                                (2py)qy_ 
    PUSH 109  %mstore_kernel_general
%endmacro
