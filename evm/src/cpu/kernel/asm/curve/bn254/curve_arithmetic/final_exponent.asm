/// To make the Tate pairing an invariant, the final step is to exponentiate by
///     (p^12 - 1)/N = (p^6 - 1) * (p^2 + 1) * (p^4 - p^2 + 1)/N
/// and thus we can exponentiate by each factor sequentially.
///
/// def bn254_final_exponent(y: Fp12):
///     y = first_exp(y)
///     y = second_exp(y)
///     return final_exp(y)

global bn254_final_exponent:

/// first, exponentiate by (p^6 - 1) via
///     def first_exp(y):
///         return y.frob(6) / y
    // stack:            k, inp, out, retdest  {out: y}
    %stack (k, inp, out) -> (out, 0, first_exp, out)         
    // stack: out, 0, first_exp, out, retdest  {out: y}
    %jump(inv_fp254_12)
first_exp:
    // stack:                           out, retdest  {out: y  , 0: y^-1}
    %frob_fp254_12_6
    // stack:                           out, retdest  {out: y_6, 0: y^-1}
    %stack (out) -> (out, 0, out, second_exp, out)
    // stack:  out, 0, out, second_exp, out, retdest  {out: y_6, 0: y^-1}
    %jump(mul_fp254_12)

/// second, exponentiate by (p^2 + 1) via 
///     def second_exp(y):
///         return y.frob(2) * y
second_exp:
    // stack:                              out, retdest  {out: y}
    %stack (out) -> (out, 0, out, out, final_exp, out)
    // stack: out, 0, out, out, final_exp, out, retdest  {out: y}
    %frob_fp254_12_2_
    // stack:      0, out, out, final_exp, out, retdest  {out: y, 0: y_2}
    %jump(mul_fp254_12)

/// Finally, we must exponentiate by (p^4 - p^2 + 1)/N
/// To do so efficiently, we can express this power as
///     (p^4 - p^2 + 1)/N = p^3 + (a2)p^2 - (a1)p - a0
/// and simultaneously compute y^a4, y^a2, y^a0 where
///     a1 = a4 + 2a2 - a0
/// We first initialize these powers as 1 and then use 
/// binary algorithms for exponentiation.
///
/// def final_exp(y):
///     y4, y2, y0 = 1, 1, 1
///     power_loop_4()
///     power_loop_2()
///     power_loop_0()
///     custom_powers()
///     final_power()

final_exp:
    // stack:                 val, retdest
    %stack (val) -> (val, 0, val)
    // stack:        val, 0, val, retdest
    %move_fp254_12
    // stack:             0, val, retdest  {0: sqr}
    %stack () -> (1, 1, 1)
    // stack:    1, 1, 1, 0, val, retdest
    %mstore_bn254_pairing(12)
    %mstore_bn254_pairing(24)
    %mstore_bn254_pairing(36)
    // stack:             0, val, retdest  {0: sqr, 12: y0, 24: y2, 36: y4}
    %stack () -> (64, 62, 65)
    // stack: 64, 62, 65, 0, val, retdest  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jump(power_loop_4)

/// After computing the powers 
///     y^a4, y^a2, y^a0
/// we would like to transform them to
///     y^a2, y^-a1, y^-a0
///
/// def custom_powers()
///     y0 = y0^{-1}
///     y1 = y4 * y2^2 * y0
///     return y2, y1, y0
///
/// And finally, upon doing so, compute the final power
///     y^(p^3) * (y^a2)^(p^2) * (y^-a1)^p * (y^-a0)
///
/// def final_power()
///     y  = y.frob(3)
///     y2 = y2.frob(2)
///     y1 = y1.frob(1)
///     return y * y2 * y1 * y0

custom_powers:
    // stack:                           val, retdest  {12: y0, 24: y2, 36: y4}
    %stack () -> (12, 48, make_term_1)
    // stack:      12, 48, make_term_1, val, retdest  {12: y0, 24: y2, 36: y4}
    %jump(inv_fp254_12)
make_term_1:
    // stack:                           val, retdest  {24: y2, 36: y4, 48: y0^-1}
    %stack () -> (24, 36, 36, make_term_2)
    // stack:  24, 36, 36, make_term_2, val, retdest  {24: y2, 36: y4, 48: y0^-1}
    %jump(mul_fp254_12)
make_term_2:
    // stack:                           val, retdest  {24: y2, 36: y4 * y2, 48: y0^-1}
    %stack () -> (24, 36, 36, make_term_3)
    // stack:  24, 36, 36, make_term_3, val, retdest  {24: y2, 36: y4 * y2, 48: y0^-1}
    %jump(mul_fp254_12)
make_term_3:
    // stack:                           val, retdest  {24: y2, 36: y4 * y2^2, 48: y0^-1}
    %stack () -> (48, 36, 36, final_power)
    // stack:  48, 36, 36, final_power, val, retdest  {24: y2, 36: y4 * y2^2, 48: y0^-1}
    %jump(mul_fp254_12)
final_power:
    // stack:                           val, retdest  {val: y  , 24:  y^a2   , 36:  y^a1   , 48: y^a0}
    %frob_fp254_12_3
    // stack:                           val, retdest  {val: y_3, 24:  y^a2   , 36:  y^a1   , 48: y^a0}
    %stack () -> (24, 24)
    %frob_fp254_12_2_
    POP
    // stack:                           val, retdest  {val: y_3, 24: (y^a2)_2, 36:  y^a1   , 48: y^a0}
    PUSH 36
    %frob_fp254_12_1
    POP
    // stack:                           val, retdest  {val: y_3, 24: (y^a2)_2, 36: (y^a1)_1, 48: y^a0}
    %stack (val) -> (24, val, val, penult_mul, val)
    // stack: 24, val, val, penult_mul, val, retdest  {val: y_3, 24: (y^a2)_2, 36: (y^a1)_1, 48: y^a0}
    %jump(mul_fp254_12)
penult_mul:
    // stack:                           val, retdest  {val: y_3 * (y^a2)_2, 36: (y^a1)_1, 48: y^a0}
    %stack (val) -> (36, val, val, final_mul, val)
    // stack:  36, val, val, final_mul, val, retdest  {val: y_3 * (y^a2)_2, 36: (y^a1)_1, 48: y^a0}
    %jump(mul_fp254_12)
final_mul: 
    // stack:                           val, retdest  {val: y_3 * (y^a2)_2 * (y^a1)_1, 48: y^a0}
    %stack (val) -> (48, val, val)
    // stack:                  48, val, val, retdest  {val: y_3 * (y^a2)_2 * (y^a1)_1, 48: y^a0}
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
    // stack:                                   i  , j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP1  
    ISZERO
    // stack:                           break?, i  , j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jumpi(power_loop_4_end)
    // stack:                                   i  , j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %sub_const(1)
    // stack:                                   i-1, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP1  
    %mload_kernel_code(power_data_4)
    // stack:                              abc, i-1, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP1  
    %lt_const(100)
    // stack:                       skip?, abc, i-1, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jumpi(power_loop_4_b)
    // stack:                              abc, i-1, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %sub_const(100)
    // stack:                               bc, i-1, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %stack () -> (36, 36, power_loop_4_b)
    // stack:      36, 36, power_loop_4_b,  bc, i-1, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP8
    // stack: sqr, 36, 36, power_loop_4_b,  bc, i-1, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jump(mul_fp254_12)
power_loop_4_b:
    // stack:                             bc, i, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP1  
    %lt_const(10)
    // stack:                      skip?, bc, i, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jumpi(power_loop_4_c)
    // stack:                             bc, i, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %sub_const(10)
    // stack:                              c, i, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %stack () -> (24, 24, power_loop_4_c)
    // stack:      24, 24, power_loop_4_c, c, i, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP8
    // stack: sqr, 24, 24, power_loop_4_c, c, i, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jump(mul_fp254_12)
power_loop_4_c:
    // stack:                            c, i, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    ISZERO
    // stack:                        skip?, i, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jumpi(power_loop_4_sq)
    // stack:                               i, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %stack () -> (12, 12, power_loop_4_sq)
    // stack:      12, 12, power_loop_4_sq, i, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP7
    // stack: sqr, 12, 12, power_loop_4_sq, i, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jump(mul_fp254_12)
power_loop_4_sq:
    // stack:                         i, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    PUSH power_loop_4  
    // stack:           power_loop_4, i, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP5  
    DUP1
    // stack: sqr, sqr, power_loop_4, i, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jump(square_fp254_12)
power_loop_4_end:
    // stack:                         0, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    POP  
    // stack:                            j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %stack () -> (36, 36, power_loop_2) 
    // stack:      36, 36, power_loop_2, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP6
    // stack: sqr, 36, 36, power_loop_2, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jump(mul_fp254_12)

power_loop_2:
    // stack:                                   j  , k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP1  
    ISZERO
    // stack:                         break?, j  , k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jumpi(power_loop_2_end)
    // stack:                                 j  , k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %sub_const(1)
    // stack:                                 j-1, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP1  
    %mload_kernel_code(power_data_2)
    // stack:                             ab, j-1, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP1  
    %lt_const(10)
    // stack:                      skip?, ab, j-1, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jumpi(power_loop_2_b)
    // stack:                             ab, j-1, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %sub_const(10)
    // stack:                              b, j-1, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %stack () -> (24, 24, power_loop_2_b) 
    // stack:      24, 24, power_loop_2_b, b, j-1, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP7
    // stack: sqr, 24, 24, power_loop_2_b, b, j-1, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jump(mul_fp254_12)
power_loop_2_b:
    // stack:                            b, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    ISZERO
    // stack:                        skip?, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jumpi(power_loop_2_sq)
    // stack:                               j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %stack () -> (12, 12, power_loop_2_sq) 
    // stack:      12, 12, power_loop_2_sq, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP6
    // stack: sqr, 12, 12, power_loop_2_sq, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jump(mul_fp254_12)
power_loop_2_sq:
    // stack:                         j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    PUSH power_loop_2  
    // stack:           power_loop_2, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP4  
    DUP1
    // stack: sqr, sqr, power_loop_2, j, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jump(square_fp254_12)
power_loop_2_end:
    // stack:                         0, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    POP  
    // stack:                            k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %stack () -> (24, 24, power_loop_0)
    // stack:      24, 24, power_loop_0, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP5
    // stack: sqr, 24, 24, power_loop_0, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jump(mul_fp254_12)

power_loop_0:
    // stack:                               k  , sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP1  
    ISZERO
    // stack:                       break?, k  , sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jumpi(power_loop_0_end)
    // stack:                               k  , sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %sub_const(1)
    // stack:                               k-1, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP1  
    %mload_kernel_code(power_data_0)
    // stack:                            a, k-1, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    ISZERO
    // stack:                        skip?, k-1, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jumpi(power_loop_0_sq)
    // stack:                               k-1, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %stack () -> (12, 12, power_loop_0_sq)  
    // stack:      12, 12, power_loop_0_sq, k-1, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP5
    // stack: sqr, 12, 12, power_loop_0_sq, k-1, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jump(mul_fp254_12)
power_loop_0_sq:
    // stack:                         k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    PUSH power_loop_0  
    // stack:           power_loop_0, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    DUP3  
    DUP1
    // stack: sqr, sqr, power_loop_0, k, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jump(square_fp254_12)
power_loop_0_end:
    // stack:                       0, sqr  {0: sqr, 12: y0, 24: y2, 36: y4}
    %stack (i, sqr) -> (12, sqr, 12, custom_powers)
    // stack:   12, sqr, 12, custom_powers  {0: sqr, 12: y0, 24: y2, 36: y4}
    %jump(mul_fp254_12)    
