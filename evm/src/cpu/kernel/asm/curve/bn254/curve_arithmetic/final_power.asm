/// To make the Tate pairing an invariant, the final step is to exponentiate by
///     (p^12 - 1)/N = (p^6 - 1)(p^2 + 1)(p^4 - p^2 + 1)/N
/// The function in this module enacts the final exponentiation, by
///     (p^4 - p^2 + 1)/N = p^3 + (a2)p^2 - (a1)p - a0
///
/// def final_exp(y):
///     y4, y2, y0 = 1, 1, 1
///     power_loop_4()
///     power_loop_2()
///     power_loop_0()
///     custom_powers()
///     final_power()
///
/// def custom_powers()
///     y0 = y0^{-1}
///     y1 = y4 * y2^2 * y0
///     return y2, y1, y0
///
/// def final_power()
///     y  = y.frob(3)
///     y2 = y2.frob(2)
///     y1 = y1.frob(1)
///     return y * y2 * y1 * y0

global bn254_final_exp:
    // stack:                  val, retdest
    %stack (val) -> (val, 300, val)
    // stack:        val, 300, val, retdest
    %move_fp12
    // stack:             300, val, retdest
    %stack () -> (1, 1, 1)
    // stack:    1, 1, 1, 300, val, retdest
    %mstore_kernel_general(200)  
    %mstore_kernel_general(224)  
    %mstore_kernel_general(212)
    // stack:             300, val, retdest  {200: y0, 212: y2, 224: y4}
    %stack () -> (64, 62, 65)
    // stack: 64, 62, 65, 300, val, retdest  {200: y0, 212: y2, 224: y4}
    %jump(power_loop_4)

custom_powers:
    // stack:                             val, retdest  {200: y0, 212: y2, 224: y4}
    %stack () -> (200, 236, make_term_1)
    // stack:      200, 236, make_term_1, val, retdest  {200: y0, 212: y2, 224: y4}
    %jump(inv_fp254_12)
make_term_1:
    // stack:                             val, retdest  {212: y2, 224: y4, 236: y0^-1}
    %stack () -> (212, 224, 224, make_term_2)
    // stack: 212, 224, 224, make_term_2, val, retdest  {212: y2, 224: y4, 236: y0^-1}
    %jump(mul_fp254_12)
make_term_2:
    // stack:                             val, retdest  {212: y2, 224: y4 * y2, 236: y0^-1}
    %stack () -> (212, 224, 224, make_term_3)
    // stack: 212, 224, 224, make_term_3, val, retdest  {212: y2, 224: y4 * y2, 236: y0^-1}
    %jump(mul_fp254_12)
make_term_3:
    // stack:                             val, retdest  {212: y2, 224: y4 * y2^2, 236: y0^-1}
    %stack () -> (236, 224, 224, final_power)
    // stack: 236, 224, 224, final_power, val, retdest  {212: y2, 224: y4 * y2^2, 236: y0^-1}
    %jump(mul_fp254_12)

final_power:
    // stack:                            val, retdest  {val: y  , 212:  y^a2   , 224:  y^a1   , 236: y^a0}
    %frob_fp254_12_3
    // stack:                            val, retdest  {val: y_3, 212:  y^a2   , 224:  y^a1   , 236: y^a0}
    %stack () -> (212, 212)
    %frob_fp254_12_2_
    POP
    // stack:                            val, retdest  {val: y_3, 212: (y^a2)_2, 224:  y^a1   , 236: y^a0}
    PUSH 224
    %frob_fp254_12_1
    POP
    // stack:                            val, retdest  {val: y_3, 212: (y^a2)_2, 224: (y^a1)_1, 236: y^a0}
    %stack (val) -> (212, val, val, penult_mul, val)
    // stack: 212, val, val, penult_mul, val, retdest  {val: y_3, 212: (y^a2)_2, 224: (y^a1)_1, 236: y^a0}
    %jump(mul_fp254_12)
penult_mul:
    // stack:                            val, retdest  {val: y_3 * (y^a2)_2, 224: (y^a1)_1, 236: y^a0}
    %stack (val) -> (224, val, val, final_mul, val)
    // stack:  224, val, val, final_mul, val, retdest  {val: y_3 * (y^a2)_2, 224: (y^a1)_1, 236: y^a0}
    %jump(mul_fp254_12)
final_mul: 
    // stack:                            val, retdest  {val: y_3 * (y^a2)_2 * (y^a1)_1, 236: y^a0}
    %stack (val) -> (236, val, val)
    // stack:                  236, val, val, retdest  {val: y_3 * (y^a2)_2 * (y^a1)_1, 236: y^a0}
    %jump(mul_fp254_12)


/// def power_loop_4():
///     for i in range(64):
///         abc = load(i, power_data_4)
///         if a:
///             y4 *= acc
///         if b:
///             y2 *= acc
///         if c:
///             y0 *= acc
///         acc = square_fp254_12(acc)
///     y4 *= acc
///
/// def power_loop_2():
///     for i in range(62):
///        ab = load(i, power_data_2)
///        if a:
///            y2 *= acc
///        if b:
///            y0 *= acc
///        acc = square_fp254_12(acc)
///     y2 *= acc
///
/// def power_loop_0():
///     for i in range(65):
///         a = load(i, power_data_0)
///         if a:
///             y0 *= acc
///         acc = square_fp254_12(acc)
///     y0 *= acc

power_loop_4:
    // stack:                                     i  , j, k, sqr  {200: y0, 212: y2, 224: y4}
    DUP1  
    ISZERO
    // stack:                             break?, i  , j, k, sqr  {200: y0, 212: y2, 224: y4}
    %jumpi(power_loop_4_end)
    // stack:                                     i  , j, k, sqr  {200: y0, 212: y2, 224: y4}
    %sub_const(1)
    // stack:                                     i-1, j, k, sqr  {200: y0, 212: y2, 224: y4}
    DUP1  
    %mload_kernel_code(power_data_4)
    // stack:                                abc, i-1, j, k, sqr  {200: y0, 212: y2, 224: y4}
    DUP1  
    %lt_const(100)
    // stack:                         skip?, abc, i-1, j, k, sqr  {200: y0, 212: y2, 224: y4}
    %jumpi(power_loop_4_b)
    // stack:                                abc, i-1, j, k, sqr  {200: y0, 212: y2, 224: y4}
    %sub_const(100)
    // stack:                                 bc, i-1, j, k, sqr  {200: y0, 212: y2, 224: y4}
    %stack () -> (224, 224, power_loop_4_b)
    // stack:      224, 224, power_loop_4_b,  bc, i-1, j, k, sqr  {200: y0, 212: y2, 224: y4}
    DUP8
    // stack: sqr, 224, 224, power_loop_4_b,  bc, i-1, j, k, sqr  {200: y0, 212: y2, 224: y4}
    %jump(mul_fp254_12)
power_loop_4_b:
    // stack:                               bc, i, j, k, sqr  {200: y0, 212: y2, 224: y4}
    DUP1  
    %lt_const(10)
    // stack:                        skip?, bc, i, j, k, sqr  {200: y0, 212: y2, 224: y4}
    %jumpi(power_loop_4_c)
    // stack:                               bc, i, j, k, sqr  {200: y0, 212: y2, 224: y4}
    %sub_const(10)
    // stack:                                c, i, j, k, sqr  {200: y0, 212: y2, 224: y4}
    %stack () -> (212, 212, power_loop_4_c)
    // stack:      212, 212, power_loop_4_c, c, i, j, k, sqr  {200: y0, 212: y2, 224: y4}
    DUP8
    // stack: sqr, 212, 212, power_loop_4_c, c, i, j, k, sqr  {200: y0, 212: y2, 224: y4}
    %jump(mul_fp254_12)
power_loop_4_c:
    // stack:                              c, i, j, k, sqr  {200: y0, 212: y2, 224: y4}
    ISZERO
    // stack:                          skip?, i, j, k, sqr  {200: y0, 212: y2, 224: y4}
    %jumpi(power_loop_4_sq)
    // stack:                                 i, j, k, sqr  {200: y0, 212: y2, 224: y4}
    %stack () -> (200, 200, power_loop_4_sq)
    // stack:      200, 200, power_loop_4_sq, i, j, k, sqr  {200: y0, 212: y2, 224: y4}
    DUP7
    // stack: sqr, 200, 200, power_loop_4_sq, i, j, k, sqr  {200: y0, 212: y2, 224: y4}
    %jump(mul_fp254_12)
power_loop_4_sq:
    // stack:                         i, j, k, sqr  {200: y0, 212: y2, 224: y4}
    PUSH power_loop_4  
    // stack:           power_loop_4, i, j, k, sqr  {200: y0, 212: y2, 224: y4}
    DUP5  
    DUP1
    // stack: sqr, sqr, power_loop_4, i, j, k, sqr  {200: y0, 212: y2, 224: y4}
    %jump(square_fp254_12)
power_loop_4_end:
    // stack:                           0, j, k, sqr  {200: y0, 212: y2, 224: y4}
    POP  
    // stack:                              j, k, sqr  {200: y0, 212: y2, 224: y4}
    %stack () -> (224, 224, power_loop_2) 
    // stack:      224, 224, power_loop_2, j, k, sqr  {200: y0, 212: y2, 224: y4}
    DUP6
    // stack: sqr, 224, 224, power_loop_2, j, k, sqr  {200: y0, 212: y2, 224: y4}
    %jump(mul_fp254_12)

power_loop_2:
    // stack:                                   j  , k, sqr  {200: y0, 212: y2, 224: y4}
    DUP1  
    ISZERO
    // stack:                           break?, j  , k, sqr  {200: y0, 212: y2, 224: y4}
    %jumpi(power_loop_2_end)
    // stack:                                   j  , k, sqr  {200: y0, 212: y2, 224: y4}
    %sub_const(1)
    // stack:                                   j-1, k, sqr  {200: y0, 212: y2, 224: y4}
    DUP1  
    %mload_kernel_code(power_data_2)
    // stack:                               ab, j-1, k, sqr  {200: y0, 212: y2, 224: y4}
    DUP1  
    %lt_const(10)
    // stack:                        skip?, ab, j-1, k, sqr  {200: y0, 212: y2, 224: y4}
    %jumpi(power_loop_2_b)
    // stack:                               ab, j-1, k, sqr  {200: y0, 212: y2, 224: y4}
    %sub_const(10)
    // stack:                                b, j-1, k, sqr  {200: y0, 212: y2, 224: y4}
    %stack () -> (212, 212, power_loop_2_b) 
    // stack:      212, 212, power_loop_2_b, b, j-1, k, sqr  {200: y0, 212: y2, 224: y4}
    DUP7
    // stack: sqr, 212, 212, power_loop_2_b, b, j-1, k, sqr  {200: y0, 212: y2, 224: y4}
    %jump(mul_fp254_12)
power_loop_2_b:
    // stack:                              b, j, k, sqr  {200: y0, 212: y2, 224: y4}
    ISZERO
    // stack:                          skip?, j, k, sqr  {200: y0, 212: y2, 224: y4}
    %jumpi(power_loop_2_sq)
    // stack:                                 j, k, sqr  {200: y0, 212: y2, 224: y4}
    %stack () -> (200, 200, power_loop_2_sq) 
    // stack:      200, 200, power_loop_2_sq, j, k, sqr  {200: y0, 212: y2, 224: y4}
    DUP6
    // stack: sqr, 200, 200, power_loop_2_sq, j, k, sqr  {200: y0, 212: y2, 224: y4}
    %jump(mul_fp254_12)
power_loop_2_sq:
    // stack:                         j, k, sqr  {200: y0, 212: y2, 224: y4}
    PUSH power_loop_2  
    // stack:           power_loop_2, j, k, sqr  {200: y0, 212: y2, 224: y4}
    DUP4  
    DUP1
    // stack: sqr, sqr, power_loop_2, j, k, sqr  {200: y0, 212: y2, 224: y4}
    %jump(square_fp254_12)
power_loop_2_end:
    // stack:                           0, k, sqr  {200: y0, 212: y2, 224: y4}
    POP  
    // stack:                              k, sqr  {200: y0, 212: y2, 224: y4}
    %stack () -> (212, 212, power_loop_0)
    // stack:      212, 212, power_loop_0, k, sqr  {200: y0, 212: y2, 224: y4}
    DUP5
    // stack: sqr, 212, 212, power_loop_0, k, sqr  {200: y0, 212: y2, 224: y4}
    %jump(mul_fp254_12)

power_loop_0:
    // stack:                                 k  , sqr  {200: y0, 212: y2, 224: y4}
    DUP1  
    ISZERO
    // stack:                         break?, k  , sqr  {200: y0, 212: y2, 224: y4}
    %jumpi(power_loop_0_end)
    // stack:                                 k  , sqr  {200: y0, 212: y2, 224: y4}
    %sub_const(1)
    // stack:                                 k-1, sqr  {200: y0, 212: y2, 224: y4}
    DUP1  
    %mload_kernel_code(power_data_0)
    // stack:                              a, k-1, sqr  {200: y0, 212: y2, 224: y4}
    ISZERO
    // stack:                          skip?, k-1, sqr  {200: y0, 212: y2, 224: y4}
    %jumpi(power_loop_0_sq)
    // stack:                                 k-1, sqr  {200: y0, 212: y2, 224: y4}
    %stack () -> (200, 200, power_loop_0_sq)  
    // stack:      200, 200, power_loop_0_sq, k-1, sqr  {200: y0, 212: y2, 224: y4}
    DUP5
    // stack: sqr, 200, 200, power_loop_0_sq, k-1, sqr  {200: y0, 212: y2, 224: y4}
    %jump(mul_fp254_12)
power_loop_0_sq:
    // stack:                         k, sqr  {200: y0, 212: y2, 224: y4}
    PUSH power_loop_0  
    // stack:           power_loop_0, k, sqr  {200: y0, 212: y2, 224: y4}
    DUP3  
    DUP1
    // stack: sqr, sqr, power_loop_0, k, sqr  {200: y0, 212: y2, 224: y4}
    %jump(square_fp254_12)
power_loop_0_end:
    // stack:                         0, sqr  {200: y0, 212: y2, 224: y4}
    %stack (i, sqr) -> (200, sqr, 200, custom_powers)
    // stack:   200, sqr, 200, custom_powers  {200: y0, 212: y2, 224: y4}
    %jump(mul_fp254_12)    
