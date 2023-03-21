// Arithmetic on integers represented with 128-bit limbs.
// These integers are represented in LITTLE-ENDIAN form.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Stores b ^ e % m in output_loc, leaving b, e, and m unchanged.
// b, e, and m must have the same length.
// output_loc must have size length and be initialized with zeroes; scratch_1 must have size length.
// All of scratch_2..scratch_5 must have size 2 * length and be initialized with zeroes.
global modexp_bignum:
    // stack: len, b_loc, e_loc, m_loc, out_loc, s1 (=scratch_1), s2, s3, s4, s5, retdest
    DUP1
    ISZERO
    %jumpi(len_zero)

    // We store the repeated-squares accumulator x_i in scratch_1, starting with x_0 := b.
    DUP1
    DUP3
    DUP8
    // stack: s1, b_loc, len, len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest
    %memcpy_kernel_general
    // stack: len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest

    // We store the accumulated output value x_i in output_loc, starting with x_0=1.
    PUSH 1
    DUP6
    // stack: out_loc, 1, len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5,  retdest
    %mstore_kernel_general

modexp_loop:
    // stack: len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest

    // y := e % 2
    DUP3
    // stack: e_loc, len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest
    %mload_kernel_general
    // stack: e_first, len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest
    %mod_const(2)
    // stack: y = e_first % 2 = e % 2, len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest
    ISZERO
    // stack: y == 0, len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest
    %jumpi(modexp_y_0)

    // if y == 1, modular-multiply output_loc by scratch_1, using scratch_2..scratch_4 as scratch space, and store in scratch_5.
    PUSH modexp_mul_return
    DUP10
    DUP10
    DUP10
    DUP14
    DUP9
    DUP12
    DUP12
    DUP9
    // stack: len, out_loc, s1, m_loc, s5, s2, s3, s4, modexp_mul_return, len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest
    %jump(modmul_bignum)
modexp_mul_return:
    // stack: len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest

    // Copy scratch_5 to output_loc.
    DUP1
    DUP11
    DUP7
    // stack: out_loc, s5, len, len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest
    %memcpy_kernel_general
    // stack: len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest

    // Zero out scratch_2..scratch_5.
    DUP1
    %mul_const(8)
    DUP8
    // stack: s2, 8 * len, len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest
    %clear_kernel_general
    // stack: len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest

modexp_y_0:
    // if y == 0, do nothing

    // Modular-square repeated-squares accumulator x_i (in scratch_1), using scratch_2..scratch_4 as scratch space, and store in scratch_5.
    PUSH modexp_square_return
    DUP10
    DUP10
    DUP10
    DUP14
    DUP9
    DUP12
    DUP1
    DUP9
    // stack: len, s1, s1, m_loc, s5, s2, s3, s4, modexp_square_return, len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest
    %jump(modmul_bignum)
    // stack: len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest

modexp_square_return:
    // Copy scratch_5 to scratch_1.
    DUP1
    DUP11
    DUP8
    // stack: s1, s5, len, len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest
    %memcpy_kernel_general
    // stack: len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest

    // Zero out scratch_2..scratch_5.
    DUP1
    %mul_const(8)
    DUP8
    // stack: s2, 8 * len, len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest
    %clear_kernel_general
    // stack: len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest

    // e //= 2 (with shr_bignum)

    PUSH modexp_shr_return
    DUP4
    DUP3
    // stack: len, e_loc, modexp_shr_return, len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest
    %jump(shr_bignum)
modexp_shr_return:
    // stack: len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest

    // check if e == 0 (with iszero_bignum)
    PUSH modexp_iszero_return
    DUP4
    DUP3
    // stack: len, e_loc, modexp_iszero_return, len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest
    %jump(iszero_bignum)
modexp_iszero_return:
    // stack: e == 0, len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest
    ISZERO
    // stack: e != 0, len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest
    %jumpi(modexp_loop)
// end of modexp_loop
    // stack: len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest
    %pop10
    // stack: retdest
    JUMP
len_zero:
    // stack: len, b_loc, e_loc, m_loc, out_loc, s1, s2, s3, s4, s5, retdest
    %pop10
    // stack: retdest
    JUMP
