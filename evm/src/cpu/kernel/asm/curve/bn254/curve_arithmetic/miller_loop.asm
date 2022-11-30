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
    PUSH 0x10  DUP2  LT       
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
    %tangent
    // stack:     line, out, mul_tangent_2, retdest, 0xnm, times, O, P, Q, out
    %sparse_store
    // stack:           out, mul_tangent_2, retdest, 0xnm, times, O, P, Q, out  {100: line}
    PUSH 100  DUP2
    // stack: out, 100, out, mul_tangent_2, retdest, 0xnm, times, O, P, Q, out  {100: line}
    %jump(mul_fp12_sparse)
mul_tangent_2:
    // stack: out, retdest, 0xnm, times,   O, P, Q, out  {100: line}
    POP  DUP5  DUP5
    // stack:   O, retdest, 0xnm, times,   O, P, Q, out  {100: line}
    // %ec_double_bn254
    // stack: 2*O, retdest, 0xnm, times,   O, P, Q, out  {100: line}
    SWAP5  SWAP1  SWAP6  SWAP1
    // stack: 2*O, retdest, 0xnm, times, 2*O, P, Q, out  {100: line}
    %pop2
    // stack:      retdest, 0xnm, times, 2*O, P, Q, out  {100: line}
    JUMP


/// def mul_cord()
///     line = cord(O, P, Q)
///     out = mul_fp12_sparse(out, line)
///     O += P

mul_cord:
    // stack:                            0xnm, times, O, P, Q, out
    PUSH mul_cord_1
    // stack:                mul_cord_1, 0xnm, times, O, P, Q, out
    DUP11  DUP11  DUP11  DUP11  DUP11  DUP11  DUP11  DUP11
    // stack:       O, P, Q, mul_cord_1, 0xnm, times, O, P, Q, out
    %cord
    // stack:          line, mul_cord_1, 0xnm, times, O, P, Q, out
    %sparse_store
    // stack:                mul_cord_1, 0xnm, times, O, P, Q, out
    DUP12
    // stack:           out, mul_cord_1, 0xnm, times, O, P, Q, out
    PUSH 100
    // stack:      100, out, mul_cord_1, 0xnm, times, O, P, Q, out
    DUP2
    // stack: out, 100, out, mul_cord_1, 0xnm, times, O, P, Q, out
    %jump(mul_fp12_sparse)
mul_cord_1:
    // stack:        0xnm, times, O  , P, Q, out
    DUP6  DUP6  DUP6  DUP6
    // stack: O , P, 0xnm, times, O  , P, Q, out
    // %ec_add_bn254
    // stack: O + P, 0xnm, times, O  , P, Q, out
    SWAP4  SWAP1  SWAP5  SWAP1
    // stack:        0xnm, times, O+P, P, Q, out
    %jump(miller_one)


%macro sparse_store
    // stack: g0, G1, G1'
    PUSH 100  %mstore_kernel_general
    // stack:     G1, G1'
    PUSH 102  %mstore_kernel_general
    PUSH 103  %mstore_kernel_general
    // stack:         G1'
    PUSH 108  %mstore_kernel_general
    PUSH 109  %mstore_kernel_general
%endmacro
