/// def power(square):
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
///     y2  = frob_fp12_2(y2)
///     return y2 * y1 * y0 

global power:
    // stack:                                           sqr, out, retdest
    PUSH 1  DUP1  DUP1
    // stack:                                  1, 1, 1, sqr, out, retdest
    %mstore_kernel_general(200)  %mstore_kernel_general(212)  %mstore_kernel_general(224)
    // stack:                                           sqr, out, retdest  {200: y0, 212: y1, 224: y2}
    PUSH power_loop_2  PUSH power_loop_1  PUSH power_return    
    // stack: power_return, power_loop_1, power_loop_2, sqr, out, retdest  {200: y0, 212: y1, 224: y2}
    SWAP3
    // stack: sqr, power_loop_1, power_loop_2, power_return, out, retdest  {200: y0, 212: y1, 224: y2}
    %jump(power_loop_0)

power_return:
    // stack:                                out, retdest  {200: y0, 212: y1, 224: y2}
    PUSH power_return_1  PUSH 236  PUSH 200
    // stack:      200, 236, power_return_1, out, retdest  {200: y0, 212: y1, 224: y2}
    %jump(inverse_fp12)
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
    %frob_fp12_2
    // stack:                           224, out, retdest  {236: y0, 212: y1, 224: y2}
    POP
    // stack:                                out, retdest  {236: y0, 212: y1, 224: y2}
    PUSH power_return_5  SWAP1
    // stack:                out, power_return_5, retdest  {236: y0, 212: y1, 224: y2}
    PUSH 236  PUSH 212
    // stack:      212, 236, out, power_return_5, retdest  {236: y0, 212: y1, 224: y2}
    %jump(mul_fp12)
power_return_5:
    // stack:                                 out, retdest  {236: y0, 212: y1, 224: y2}
    PUSH 224  DUP2
    // stack:                       out, 224, out, retdest  {236: y0, 212: y1, 224: y2}
    %jump(mul_fp12)

/// def power_loop_0():
///     for i in range(1, len4):
///         if EXP4[-i]:
///             y1 *= square
///         if EXP2[-i]:
///             y2 *= square
///         if EXP0[-i]:
///             y0 *= square
///         square = square_fp12(square)
///     y1 *= square
///
/// def power_loop_1():
///     for i in range(len4, len2):
///        if EXP2[-i]:
///            y2 *= square
///        if EXP0[-i]:
///            y0 *= square
///        square = square_fp12(square)
///     y2 *= square
///
/// def power_loop_2():
///     for i in range(len2, len0):
///         if EXP0[-i]:
///             y0 *= square
///         square = square_fp12(square)
///     y0 *= square



