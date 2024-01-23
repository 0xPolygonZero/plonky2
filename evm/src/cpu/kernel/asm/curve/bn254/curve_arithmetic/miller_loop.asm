/// def miller(P, Q):
///     miller_init()
///     miller_loop()
///
/// def miller_init():
///     out = 1
///     O = P
///     times = 61
///
/// def miller_loop():
///     while times:
///         0xnm = load(miller_data)
///         while 0xnm > 0x20:
///             miller_one()
///         while 0xnm:
///             miller_zero()
///         times -= 1
///
/// def miller_one():
///     0xnm -= 0x20
///     mul_tangent()
///     mul_cord()
///
/// def miller_zero():
///     0xnm -= 1
///     mul_tangent()

global bn254_miller:
    // stack:            ptr, out, retdest
    %stack (ptr, out) -> (out, ptr, out)
    // stack: out, ptr, out, retdest
    %write_fp254_12_unit
    // stack:            ptr, out, retdest
    %load_fp254_6
    // stack:           P, Q, out, retdest
    %stack (P: 2) -> (0, 53, P, P)
    // stack: 0, 53, O, P, Q, out, retdest
    // the head 0 lets miller_loop start with POP
miller_loop:
    POP
    // stack:          times  , O, P, Q, out, retdest
    DUP1
    ISZERO
    // stack:  break?, times  , O, P, Q, out, retdest
    %jumpi(miller_return)
    // stack:          times  , O, P, Q, out, retdest
    %sub_const(1)
    // stack:          times-1, O, P, Q, out, retdest
    DUP1
    // stack: times-1, times-1, O, P, Q, out, retdest
    %mload_kernel_code(miller_data)
    // stack:    0xnm, times-1, O, P, Q, out, retdest
    %jump(miller_one)
miller_return:
    // stack: times, O, P, Q, out, retdest
    %stack (times, O: 2, P: 2, Q: 4, out, retdest) -> (retdest)
    // stack:                      retdest
    %clear_line
    JUMP 

miller_one:
    // stack:               0xnm, times, O, P, Q, out, retdest
    DUP1
    %lt_const(0x20) 
    // stack:        skip?, 0xnm, times, O, P, Q, out, retdest
    %jumpi(miller_zero)
    // stack:               0xnm, times, O, P, Q, out, retdest
    %sub_const(0x20)
    // stack:           0x{n-1}m, times, O, P, Q, out, retdest
    PUSH mul_cord
    // stack: mul_cord, 0x{n-1}m, times, O, P, Q, out, retdest
    %jump(mul_tangent)

miller_zero:
    // stack:              m  , times, O, P, Q, out, retdest
    DUP1
    ISZERO
    // stack:       skip?, m  , times, O, P, Q, out, retdest
    %jumpi(miller_loop)
    // stack:              m  , times, O, P, Q, out, retdest
    %sub_const(1)
    // stack:              m-1, times, O, P, Q, out, retdest
    PUSH miller_zero
    // stack: miller_zero, m-1, times, O, P, Q, out, retdest
    %jump(mul_tangent)


/// def mul_tangent()
///     out = square_fp254_12(out)
///     line = tangent(O, Q)
///     out = mul_fp254_12_sparse(out, line)
///     O += O

mul_tangent:
    // stack:                                              retdest, 0xnm, times, O, P, Q, out
    PUSH mul_tangent_2
    DUP13
    PUSH mul_tangent_1
    // stack:           mul_tangent_1, out, mul_tangent_2, retdest, 0xnm, times, O, P, Q, out
    %stack (mul_tangent_1, out) -> (out, out, mul_tangent_1, out)
    // stack: out, out, mul_tangent_1, out, mul_tangent_2, retdest, 0xnm, times, O, P, Q, out
    %jump(square_fp254_12)
mul_tangent_1:
    // stack:          out, mul_tangent_2, retdest, 0xnm, times, O, P, Q, out
    DUP13
    DUP13
    DUP13
    DUP13
    // stack:       Q, out, mul_tangent_2, retdest, 0xnm, times, O, P, Q, out
    DUP11
    DUP11
    // stack:    O, Q, out, mul_tangent_2, retdest, 0xnm, times, O, P, Q, out
    %tangent
    // stack:          out, mul_tangent_2, retdest, 0xnm, times, O, P, Q, out  {12: line}
    %stack (out) -> (out, 12, out)
    // stack: out, 12, out, mul_tangent_2, retdest, 0xnm, times, O, P, Q, out  {12: line}
    %jump(mul_fp254_12_sparse)
mul_tangent_2:
    // stack:                  retdest, 0xnm, times,   O, P, Q, out  {12: line}
    PUSH after_double
    // stack:    after_double, retdest, 0xnm, times,   O, P, Q, out  {12: line}
    DUP6  
    DUP6
    // stack: O, after_double, retdest, 0xnm, times,   O, P, Q, out  {12: line}
    %jump(bn_double)
after_double:
    // stack:             2*O, retdest, 0xnm, times,   O, P, Q, out  {12: line}
    SWAP5
    POP
    SWAP5
    POP
    // stack:                  retdest, 0xnm, times, 2*O, P, Q, out  {12: line}
    JUMP

/// def mul_cord()
///     line = cord(P, O, Q)
///     out = mul_fp254_12_sparse(out, line)
///     O += P

mul_cord:
    // stack:                           0xnm, times, O, P, Q, out
    PUSH mul_cord_1
    // stack:               mul_cord_1, 0xnm, times, O, P, Q, out
    DUP11
    DUP11
    DUP11
    DUP11
    // stack:            Q, mul_cord_1, 0xnm, times, O, P, Q, out
    DUP9
    DUP9
    // stack:         O, Q, mul_cord_1, 0xnm, times, O, P, Q, out
    DUP13
    DUP13
    // stack:      P, O, Q, mul_cord_1, 0xnm, times, O, P, Q, out
    %cord 
    // stack:               mul_cord_1, 0xnm, times, O, P, Q, out  {12: line}
    DUP12
    // stack:          out, mul_cord_1, 0xnm, times, O, P, Q, out  {12: line}
    %stack (out) -> (out, 12, out)
    // stack: out, 12, out, mul_cord_1, 0xnm, times, O, P, Q, out  {12: line}
    %jump(mul_fp254_12_sparse)
mul_cord_1:
    // stack:                   0xnm, times, O  , P, Q, out
    PUSH after_add
    // stack:        after_add, 0xnm, times, O  , P, Q, out
    DUP7  
    DUP7  
    DUP7  
    DUP7
    // stack: O , P, after_add, 0xnm, times, O  , P, Q, out
    %jump(bn_add_valid_points)
after_add:
    // stack:            O + P, 0xnm, times, O  , P, Q, out
    SWAP4
    POP
    SWAP4
    POP
    // stack:                   0xnm, times, O+P, P, Q, out
    %jump(miller_one)


/// def tangent(px, py, qx, qy):
///     return sparse_store(
///         py**2 - 9, 
///         (-3px**2) * qx, 
///         (2py)     * qy,
///     )

%macro tangent
    // stack:                px, py, qx, qx_,  qy, qy_
    PUSH 12
    %create_bn254_pairing_address
    %stack (addr12, px, py) -> (py, py, 9, addr12, addr12, px, py)
    // stack:    py, py, 9, addr12, addr12, px, py, qx, qx_,  qy, qy_
    MULFP254
    // stack:      py^2, 9, addr12, addr12, px, py, qx, qx_,  qy, qy_
    SUBFP254
    // stack:      py^2 - 9, addr12, addr12, px, py, qx, qx_,  qy, qy_
    MSTORE_GENERAL
    // stack:               addr12, px, py, qx, qx_,  qy, qy_
    %add_const(2) DUP1
    SWAP2
    DUP1
    MULFP254
    // stack:              px^2, addr14, addr14, py, qx, qx_,  qy, qy_
    PUSH 3
    MULFP254
    // stack:            3*px^2, addr14, addr14, py, qx, qx_,  qy, qy_
    PUSH 0
    SUBFP254
    // stack:           -3*px^2, addr14, addr14, py, qx, qx_,  qy, qy_
    SWAP4
    // stack:            qx, addr14, addr14, py, -3px^2, qx_,  qy, qy_
    DUP5
    MULFP254
    // stack:   (-3*px^2)qx, addr14, addr14, py, -3px^2, qx_,  qy, qy_
    MSTORE_GENERAL
    // stack:                addr14, py, -3px^2, qx_,  qy, qy_ 
    DUP1 %add_const(6)
    // stack:                addr20, addr14, py, -3px^2, qx_,  qy, qy_
    %stack (addr20, addr14, py) -> (2, py, addr20, addr14)
    MULFP254
    // stack:               2py, addr20, addr14, -3px^2, qx_,  qy, qy_ 
    SWAP5
    // stack:                qy, addr20, addr14, -3px^2, qx_, 2py, qy_ 
    DUP6
    MULFP254
    // stack:           (2py)qy, addr20, addr14, -3px^2, qx_, 2py, qy_ 
    MSTORE_GENERAL
    // stack:                   addr14, -3px^2, qx_, 2py, qy_
    %add_const(1) SWAP2
    // stack:                   qx_, -3px^2, addr15, 2py, qy_
    MULFP254
    // stack:                   (-3px^2)*qx_, addr15, 2py, qy_ 
    MSTORE_GENERAL
    // stack:                                 2py, qy_ 
    MULFP254
    // stack:                                (2py)*qy_ 
    %mstore_bn254_pairing(21)
%endmacro

/// def cord(p1x, p1y, p2x, p2y, qx, qy):
///     return sparse_store(
///         p1y*p2x - p2y*p1x, 
///         (p2y - p1y) * qx, 
///         (p1x - p2x) * qy,
///     )

%macro cord
    // stack:                    p1x , p1y, p2x , p2y, qx, qx_, qy, qy_
    DUP1
    DUP5
    MULFP254
    // stack:           p2y*p1x, p1x , p1y, p2x , p2y, qx, qx_, qy, qy_
    DUP3
    DUP5  
    MULFP254
    // stack: p1y*p2x , p2y*p1x, p1x , p1y, p2x , p2y, qx, qx_, qy, qy_
    SUBFP254
    // stack: p1y*p2x - p2y*p1x, p1x , p1y, p2x , p2y, qx, qx_, qy, qy_
    %mstore_bn254_pairing(12)
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
    DUP5
    MULFP254
    // stack:         (p1x - p2x)qy, p2y - p1y, qx, qx_, p1x - p2x, qy_
    %mstore_bn254_pairing(20)
    // stack:                        p2y - p1y, qx, qx_, p1x - p2x, qy_
    SWAP1
    // stack:                        qx, p2y - p1y, qx_, p1x - p2x, qy_
    DUP2
    MULFP254
    // stack:             (p2y - p1y)qx, p2y - p1y, qx_, p1x - p2x, qy_
    %mstore_bn254_pairing(14)
    // stack:                            p2y - p1y, qx_, p1x - p2x, qy_
    MULFP254
    // stack:                            (p2y - p1y)qx_, p1x - p2x, qy_
    %mstore_bn254_pairing(15)
    // stack:                                            p1x - p2x, qy_
    MULFP254
    // stack:                                           (p1x - p2x)*qy_
    %mstore_bn254_pairing(21)
%endmacro

%macro clear_line
    PUSH 12
    %create_bn254_pairing_address
    // stack: addr12
    DUP1 %add_const(2)
    // stack: addr14, addr12
    DUP1 %add_const(1)
    // stack: addr15, addr14, addr12
    DUP1 %add_const(5)
    // stack: addr20, addr15, addr14, addr12
    DUP1 %add_const(1)
    // stack: addr21, addr20, addr15, addr14, addr12
    %rep 5
        PUSH 0 MSTORE_GENERAL
    %endrep
%endmacro


%macro write_fp254_12_unit
    // Write 0x10000000000000000000000 with MSTORE_32BYTES_12,
    // effectively storing 1 at the initial offset, and 11 0s afterwards.

    // stack: out
    %create_bn254_pairing_address
    // stack: addr
    PUSH 0x10000000000000000000000
    SWAP1
    // stack: addr, 0x10000000000000000000000
    MSTORE_32BYTES_12
    POP
    // stack:
%endmacro
