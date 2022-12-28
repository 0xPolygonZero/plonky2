global test_pow:
    // stack: ptr, f, ptr, out, ret_stack, out
    %store_fp12
    // stack:         ptr, out, ret_stack, out
    %jump(power)

/// def power(acc):
///     power_init()
///     power_loop_4()
///     power_loop_2()
///     power_loop_0()
///     power_return()
///
/// def power_init()
///     y0, y4, y2 = 1, 1, 1
///
/// def power_return()
///     y0  = y0^{-1}
///     y4 *= y0 * (y2**2)
///     y4  = frob_fp12_1(y4)
///     y2  = frob_fp12_2_(y2)
///     return y2 * y4 * y0 

global power:
    // stack:             ptr, out, retdest
    PUSH 1  DUP1  DUP1
    // stack:    1, 1, 1, ptr, out, retdest
    %mstore_kernel_general(200)  %mstore_kernel_general(224)  %mstore_kernel_general(212)
    // stack:             ptr, out, retdest  {200: y0, 212: y2, 224: y4}
    PUSH 65  PUSH 62  PUSH 65
    // stack: 65, 62, 65, ptr, out, retdest  {200: y0, 212: y2, 224: y4}
    %jump(power_loop_4)

power_return:
    // stack:                                out, retdest  {200: y0, 212: y2, 224: y4}
    PUSH power_return_1  PUSH 236  PUSH 200
    // stack:      200, 236, power_return_1, out, retdest  {200: y0, 212: y2, 224: y4}
    %jump(inv_fp12)
power_return_1:
    // stack:                                out, retdest  {236: y0, 212: y2, 224: y4}
    PUSH power_return_2  PUSH 224  DUP1  PUSH 212
    // stack: 212, 224, 224, power_return_2, out, retdest  {236: y0, 212: y2, 224: y4}
    %jump(mul_fp12)
power_return_2: 
    // stack:                                out, retdest  {236: y0, 212: y2, 224: y4}
    PUSH power_return_3  PUSH 224  DUP1  PUSH 212
    // stack: 212, 224, 224, power_return_3, out, retdest  {236: y0, 212: y2, 224: y4}
    %jump(mul_fp12)
power_return_3:
    // stack:                                out, retdest  {236: y0, 212: y2, 224: y4}
    PUSH power_return_4  PUSH 224  DUP1  PUSH 236
    // stack: 236, 224, 224, power_return_4, out, retdest  {236: y0, 212: y2, 224: y4}
    %jump(mul_fp12)
power_return_4:
    // stack:                                out, retdest  {236: y0, 212: y2, 224: y4}
    PUSH 224
    // stack:                           224, out, retdest  {236: y0, 212: y2, 224: y4}
    %frob_fp12_1
    // stack:                           224, out, retdest  {236: y0, 212: y2, 224: y4}
    POP
    // stack:                                out, retdest  {236: y0, 212: y2, 224: y4}
    PUSH 212  DUP1
    // stack:                      212, 212, out, retdest  {236: y0, 212: y2, 224: y4}
    %frob_fp12_2_
    // stack:                           212, out, retdest  {236: y0, 212: y2, 224: y4}
    POP
    // stack:                                out, retdest  {236: y0, 212: y2, 224: y4}
    PUSH power_return_5  DUP2  PUSH 236  PUSH 224
    // stack: 224, 236, out, power_return_5, out, retdest  {236: y0, 212: y2, 224: y4}
    %jump(mul_fp12)
power_return_5:
    // stack:                                out, retdest  {236: y0, 212: y2, 224: y4}
    PUSH 212  DUP2
    // stack:                      out, 212, out, retdest  {236: y0, 212: y2, 224: y4}
    %jump(mul_fp12)

/// def power_loop_4():
///     for i in range(65):
///         abc = load(i, power_data_4)
///         if a:
///             y4 *= acc
///         if b:
///             y2 *= acc
///         if c:
///             y0 *= acc
///         acc = square_fp12(acc)
///     y4 *= acc
///
/// def power_loop_2():
///     for i in range(62):
///        ab = load(i, power_data_2)
///        if a:
///            y2 *= acc
///        if b:
///            y0 *= acc
///        acc = square_fp12(acc)
///     y2 *= acc
///
/// def power_loop_0():
///     for i in range(65):
///         a = load(i, power_data_0)
///         if a:
///             y0 *= acc
///         acc = square_fp12(acc)
///     y0 *= acc

power_loop_4:
    // stack:                                     i  , j, k, ptr  {200: y0, 212: y2, 224: y4}
    DUP1  ISZERO
    // stack:                             break?, i  , j, k, ptr  {200: y0, 212: y2, 224: y4}
    %jumpi(power_loop_4_end)
    // stack:                                     i  , j, k, ptr  {200: y0, 212: y2, 224: y4}
    %sub_const(1)
    // stack:                                     i-1, j, k, ptr  {200: y0, 212: y2, 224: y4}
    DUP1  %mload_kernel_code(power_data_4)
    // stack:                                abc, i-1, j, k, ptr  {200: y0, 212: y2, 224: y4}
    DUP1  %lt_const(100)
    // stack:                         skip?, abc, i-1, j, k, ptr  {200: y0, 212: y2, 224: y4}
    %jumpi(power_loop_4_b)
    // stack:                                abc, i-1, j, k, ptr  {200: y0, 212: y2, 224: y4}
    %sub_const(100)
    // stack:                                 bc, i-1, j, k, ptr  {200: y0, 212: y2, 224: y4}
    PUSH power_loop_4_b  PUSH 224  DUP1  DUP8
    // stack: ptr, 224, 224, power_loop_4_b,  bc, i-1, j, k, ptr  {200: y0, 212: y2, 224: y4}
    %jump(mul_fp12)
power_loop_4_b:
    // stack:                               bc, i, j, k, ptr  {200: y0, 212: y2, 224: y4}
    DUP1  %lt_const(10)
    // stack:                        skip?, bc, i, j, k, ptr  {200: y0, 212: y2, 224: y4}
    %jumpi(power_loop_4_c)
    // stack:                               bc, i, j, k, ptr  {200: y0, 212: y2, 224: y4}
    %sub_const(10)
    // stack:                                c, i, j, k, ptr  {200: y0, 212: y2, 224: y4}
    PUSH power_loop_4_c  PUSH 212  DUP1  DUP8
    // stack: ptr, 212, 212, power_loop_4_c, c, i, j, k, ptr  {200: y0, 212: y2, 224: y4}
    %jump(mul_fp12)
power_loop_4_c:
    // stack:                              c, i, j, k, ptr  {200: y0, 212: y2, 224: y4}
    ISZERO
    // stack:                          skip?, i, j, k, ptr  {200: y0, 212: y2, 224: y4}
    %jumpi(power_loop_4_sq)
    // stack:                                 i, j, k, ptr  {200: y0, 212: y2, 224: y4}
    PUSH power_loop_4_sq  PUSH 200  DUP1  DUP7
    // stack: ptr, 200, 200, power_loop_4_sq, i, j, k, ptr  {200: y0, 212: y2, 224: y4}
    %jump(mul_fp12)
power_loop_4_sq:
    // stack:                         i, j, k, ptr  {200: y0, 212: y2, 224: y4}
    PUSH power_loop_4  DUP5  DUP1
    // stack: ptr, ptr, power_loop_4, i, j, k, ptr  {200: y0, 212: y2, 224: y4}
    %jump(square_fp12)
power_loop_4_end:
    // stack:                           0, j, k, ptr  {200: y0, 212: y2, 224: y4}
    POP  
    // stack:                              j, k, ptr  {200: y0, 212: y2, 224: y4}
    PUSH power_loop_2  PUSH 224  DUP1  DUP6
    // stack: ptr, 224, 224, power_loop_2, j, k, ptr  {200: y0, 212: y2, 224: y4}
    %jump(mul_fp12)

power_loop_2:
    // stack:                                   j  , k, ptr  {200: y0, 212: y2, 224: y4}
    DUP1  ISZERO
    // stack:                           break?, j  , k, ptr  {200: y0, 212: y2, 224: y4}
    %jumpi(power_loop_2_end)
    // stack:                                   j  , k, ptr  {200: y0, 212: y2, 224: y4}
    %sub_const(1)
    // stack:                                   j-1, k, ptr  {200: y0, 212: y2, 224: y4}
    DUP1  %mload_kernel_code(power_data_2)
    // stack:                               ab, j-1, k, ptr  {200: y0, 212: y2, 224: y4}
    DUP1  %lt_const(10)
    // stack:                        skip?, ab, j-1, k, ptr  {200: y0, 212: y2, 224: y4}
    %jumpi(power_loop_2_b)
    // stack:                               ab, j-1, k, ptr  {200: y0, 212: y2, 224: y4}
    %sub_const(10)
    // stack:                                b, j-1, k, ptr  {200: y0, 212: y2, 224: y4}
    PUSH power_loop_2_b  PUSH 212  DUP1  DUP7
    // stack: ptr, 212, 212, power_loop_2_b, b, j-1, k, ptr  {200: y0, 212: y2, 224: y4}
    %jump(mul_fp12)
power_loop_2_b:
    // stack:                              b, j, k, ptr  {200: y0, 212: y2, 224: y4}
    ISZERO
    // stack:                          skip?, j, k, ptr  {200: y0, 212: y2, 224: y4}
    %jumpi(power_loop_2_sq)
    // stack:                                 j, k, ptr  {200: y0, 212: y2, 224: y4}
    PUSH power_loop_2_sq  PUSH 200  DUP1  DUP6
    // stack: ptr, 200, 200, power_loop_2_sq, j, k, ptr  {200: y0, 212: y2, 224: y4}
    %jump(mul_fp12)
power_loop_2_sq:
    // stack:                         j, k, ptr  {200: y0, 212: y2, 224: y4}
    PUSH power_loop_2  DUP4  DUP1
    // stack: ptr, ptr, power_loop_2, j, k, ptr  {200: y0, 212: y2, 224: y4}
    %jump(square_fp12)
power_loop_2_end:
    // stack:                           0, k, ptr  {200: y0, 212: y2, 224: y4}
    POP  
    // stack:                              k, ptr  {200: y0, 212: y2, 224: y4}
    PUSH power_loop_0  PUSH 212  DUP1  DUP5
    // stack: ptr, 212, 212, power_loop_0, k, ptr  {200: y0, 212: y2, 224: y4}
    %jump(mul_fp12)


power_loop_0:
    // stack:                                 k  , ptr
    DUP1  ISZERO
    // stack:                         break?, k  , ptr
    %jumpi(power_loop_0_end)
    // stack:                                 k  , ptr
    %sub_const(1)
    // stack:                                 k-1, ptr
    DUP1  %mload_kernel_code(power_data_0)
    // stack:                              a, k-1, ptr
    ISZERO
    // stack:                          skip?, k-1, ptr
    %jumpi(power_loop_0_sq)
    // stack:                                 k-1, ptr
    PUSH power_loop_0_sq  PUSH 200  DUP1  DUP5
    // stack: ptr, 200, 200, power_loop_0_sq, k-1, ptr
    %jump(mul_fp12)
power_loop_0_sq:
    // stack:                         k, ptr
    PUSH power_loop_0  DUP3  DUP1
    // stack: ptr, ptr, power_loop_0, k, ptr
    %jump(square_fp12)
power_loop_0_end:
    // stack:                      0, ptr
    POP  
    // stack:                         ptr
    PUSH 200  PUSH power_return  SWAP2  DUP2 
    // stack: 200, ptr, 200, power_return
    %jump(mul_fp12)
