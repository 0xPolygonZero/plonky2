global test_pow:
    // stack: ptr, f, ptr, out, ret_stack, out
    %store_fp12
    // stack:         ptr, out, ret_stack, out
    %jump(power)

/// def power(acc):
///     power_init()
///     power_loop_0()
///     power_loop_1()
///     power_loop_2()
///     power_return()
///
/// def power_init()
///     y0, y1, y2 = 1, 1, 1
///
/// def power_return()
///     y0  = y0^{-1}
///     y1 *= y0 * (y2**2)
///     y1  = frob_fp12_1(y1)
///     y2  = frob_fp12_2_(y2)
///     return y2 * y1 * y0 

global power:
    // stack:                                                       ptr, out, retdest
    PUSH 1  DUP1  DUP1
    // stack:                                              1, 1, 1, ptr, out, retdest
    %mstore_kernel_general(200)  %mstore_kernel_general(212)  %mstore_kernel_general(224)
    // stack:                                                       ptr, out, retdest  {200: y0, 212: y1, 224: y2}
    PUSH power_loop_2  PUSH power_loop_1  PUSH power_return    
    // stack:             power_return, power_loop_1, power_loop_2, ptr, out, retdest  {200: y0, 212: y1, 224: y2}
    SWAP3
    // stack:             ptr, power_loop_1, power_loop_2, power_return, out, retdest  {200: y0, 212: y1, 224: y2}
    PUSH 65  PUSH 62  PUSH 65
    // stack: 65, 62, 65, ptr, power_loop_1, power_loop_2, power_return, out, retdest  {200: y0, 212: y1, 224: y2}
    %jump(power_loop_0)

power_return:
    // stack:                                out, retdest  {200: y0, 212: y1, 224: y2}
    PUSH power_return_1  PUSH 236  PUSH 200
    // stack:      200, 236, power_return_1, out, retdest  {200: y0, 212: y1, 224: y2}
    %jump(inv_fp12)
power_return_1:
    // stack:                                out, retdest  {236: y0, 212: y1, 224: y2}
    PUSH power_return_2  PUSH 248  PUSH 224
    // stack:      224, 248, power_return_2, out, retdest  {200: y0, 212: y1, 224: y2}
    %jump(square_fp12)
power_return_2:
    // stack:                                out, retdest  {236: y0, 212: y1, 224: y2, 248: y2^2}
    PUSH power_return_3  PUSH 248  PUSH 224  PUSH 248
    // stack: 248, 236, 248, power_return_3, out, retdest  {236: y0, 212: y1, 224: y2, 248: y2^2}
    %jump(mul_fp12)
power_return_3:
    // stack:                                out, retdest  {236: y0, 212: y1, 224: y2, 248: y0*y2^2}
    PUSH power_return_4  PUSH 212  PUSH 248  PUSH 212
    // stack: 212, 248, 212, power_return_4, out, retdest  {236: y0, 212: y1, 224: y2, 248: y0*y2^2}
    %jump(mul_fp12)
power_return_4:
    // stack:                                out, retdest  {236: y0, 212: y1, 224: y2}
    PUSH 212
    // stack:                           212, out, retdest  {236: y0, 212: y1, 224: y2}
    %frob_fp12_1
    // stack:                           212, out, retdest  {236: y0, 212: y1, 224: y2}
    POP
    // stack:                                out, retdest  {236: y0, 212: y1, 224: y2}
    PUSH 224  DUP1
    // stack:                      224, 224, out, retdest  {236: y0, 212: y1, 224: y2}
    %frob_fp12_2_
    // stack:                           224, out, retdest  {236: y0, 212: y1, 224: y2}
    POP
    // stack:                                out, retdest  {236: y0, 212: y1, 224: y2}
    PUSH power_return_5  SWAP1
    // stack:                out, power_return_5, retdest  {236: y0, 212: y1, 224: y2}
    PUSH 236  PUSH 212
    // stack:      212, 236, out, power_return_5, retdest  {236: y0, 212: y1, 224: y2}
    %jump(mul_fp12)
power_return_5:
    // stack:                                out, retdest  {236: y0, 212: y1, 224: y2}
    PUSH 224  DUP2
    // stack:                      out, 224, out, retdest  {236: y0, 212: y1, 224: y2}
    %jump(mul_fp12)

/// def power_loop_0():
///     for i in range(1, len4):
///         abc = load(power_data_0)
///         if a:
///             y1 *= acc
///         if b:
///             y2 *= acc
///         if c:
///             y0 *= acc
///         acc = square_fp12(acc)
///     y1 *= acc
///
/// def power_loop_1():
///     for i in range(len4, len2):
///        ab = load(power_data_1)
///        if a:
///            y2 *= acc
///        if b:
///            y0 *= acc
///        acc = square_fp12(acc)
///     y2 *= acc
///
/// def power_loop_2():
///     for i in range(len2, len0):
///         a = load(power_data_1)
///         if a:
///             y0 *= acc
///         acc = square_fp12(acc)
///     y0 *= acc

power_loop_0:
    // stack:                                     i  , j, k, ptr, retdest
    DUP1  ISZERO
    // stack:                             break?, i  , j, k, ptr, retdest
    %jumpi(power_loop_0_end)
    // stack:                                     i  , j, k, ptr, retdest
    %sub_const(1)
    // stack:                                     i-1, j, k, ptr, retdest
    DUP1  %mload_kernel_code(power_data_0)
    // stack:                                abc, i-1, j, k, ptr, retdest
    DUP1  %lt_const(100)
    // stack:                         skip?, abc, i-1, j, k, ptr, retdest
    %jumpi(power_loop_0_b)
    // stack:                                abc, i-1, j, k, ptr, retdest
    %sub_const(100)
    // stack:                                 bc, i-1, j, k, ptr, retdest
    PUSH power_loop_0_b  PUSH 212  DUP1  DUP8
    // stack: ptr, 212, 212, power_loop_0_b,  bc, i-1, j, k, ptr, retdest
    %jump(mul_fp12)
power_loop_0_b:
    // stack:                               bc, i, j, k, ptr, retdest
    DUP1  %lt_const(10)
    // stack:                        skip?, bc, i, j, k, ptr, retdest
    %jumpi(power_loop_0_c)
    // stack:                               bc, i, j, k, ptr, retdest
    %sub_const(10)
    // stack:                                c, i, j, k, ptr, retdest
    PUSH power_loop_0_c  PUSH 224  DUP1  DUP8
    // stack: ptr, 224, 224, power_loop_0_c, c, i, j, k, ptr, retdest
    %jump(mul_fp12)
power_loop_0_c:
    // stack:                              c, i, j, k, ptr, retdest
    DUP1  ISZERO
    // stack:                       skip?, c, i, j, k, ptr, retdest
    %jumpi(power_loop_0_sq)
    // stack:                              c, i, j, k, ptr, retdest
    POP
    // stack:                                 i, j, k, ptr, retdest
    PUSH power_loop_0_sq  PUSH 200  DUP1  DUP7
    // stack: ptr, 200, 200, power_loop_0_sq, i, j, k, ptr, retdest
    %jump(mul_fp12)
power_loop_0_sq:
    // stack:                         i, j, k, ptr, retdest
    PUSH power_loop_0  DUP5  DUP1
    // stack: ptr, ptr, power_loop_0, i, j, k, ptr, retdest
    %jump(mul_fp12)
power_loop_0_end:
    // stack:                           0, j, k, ptr, retdest
    POP  
    // stack:                              j, k, ptr, retdest
    PUSH power_loop_1  PUSH 212  DUP1  DUP6
    // stack: ptr, 212, 212, power_loop_1, j, k, ptr, retdest
    %jump(mul_fp12)

power_loop_1:
    // stack:                                   j  , k, ptr, retdest
    DUP1  ISZERO
    // stack:                           break?, j  , k, ptr, retdest
    %jumpi(power_loop_1_end)
    // stack:                                   j  , k, ptr, retdest
    %sub_const(1)
    // stack:                                   j-1, k, ptr, retdest
    DUP1  %mload_kernel_code(power_data_1)
    // stack:                               ab, j-1, k, ptr, retdest
    DUP1  %lt_const(10)
    // stack:                        skip?, ab, j-1, k, ptr, retdest
    %jumpi(power_loop_1_b)
    // stack:                               ab, j-1, k, ptr, retdest
    %sub_const(10)
    // stack:                                b, j-1, k, ptr, retdest
    PUSH power_loop_1_b  PUSH 224  DUP1  DUP7
    // stack: ptr, 224, 224, power_loop_1_b, b, j-1, k, ptr, retdest
    %jump(mul_fp12)
power_loop_1_b:
    // stack:                              b, j, k, ptr, retdest
    DUP1  ISZERO
    // stack:                       skip?, b, j, k, ptr, retdest
    %jumpi(power_loop_1_sq)
    // stack:                              b, j, k, ptr, retdest
    POP
    // stack:                                 j, k, ptr, retdest
    PUSH power_loop_1_sq  PUSH 200  DUP1  DUP6
    // stack: ptr, 200, 200, power_loop_1_sq, j, k, ptr, retdest
    %jump(mul_fp12)
power_loop_1_sq:
    // stack:                         j, k, ptr, retdest
    PUSH power_loop_1  DUP4  DUP1
    // stack: ptr, ptr, power_loop_1, j, k, ptr, retdest
    %jump(square_fp12)
power_loop_1_end:
    // stack:                           0, k, ptr, retdest
    POP  
    // stack:                              k, ptr, retdest
    PUSH power_loop_2  PUSH 224  DUP1  DUP6
    // stack: ptr, 224, 224, power_loop_2, k, ptr, retdest
    %jump(mul_fp12)


power_loop_2:
    // stack:                                 k  , ptr, retdest
    DUP1  ISZERO
    // stack:                         break?, k  , ptr, retdest
    %jumpi(power_loop_2_end)
    // stack:                                 k  , ptr, retdest
    %sub_const(1)
    // stack:                                 k-1, ptr, retdest
    DUP1  %mload_kernel_code(power_data_2)
    // stack:                              a, k-1, ptr, retdest
    DUP1  ISZERO
    // stack:                       skip?, a, k-1, ptr, retdest
    %jumpi(power_loop_2_sq)
    // stack:                              a, k-1, ptr, retdest
    POP
    // stack:                                 k-1, ptr, retdest
    PUSH power_loop_2_sq  PUSH 200  DUP1  DUP5
    // stack: ptr, 200, 200, power_loop_2_sq, k-1, ptr, retdest
    %jump(mul_fp12)
power_loop_2_sq:
    // stack:                         k, ptr, retdest
    PUSH power_loop_2  DUP3  DUP1
    // stack: ptr, ptr, power_loop_2, k, ptr, retdest
    %jump(square_fp12)
power_loop_2_end:
    // stack:                           0, ptr, retdest
    POP  
    // stack:                              ptr, retdest
    PUSH power_return  PUSH 200  DUP1  DUP4
    // stack: ptr, 200, 200, power_return, ptr, retdest
    %jump(mul_fp12)
