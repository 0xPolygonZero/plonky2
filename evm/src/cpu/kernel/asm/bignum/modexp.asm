// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Stores b ^ e % m in output_loc, leaving b, e, and m unchanged.
// b, e, and m must have the same length.
// output_loc must have size length and be initialized with zeroes; scratch_1 must have size length.
// All of scratch_2..scratch_6 must have size 2 * length and be initialized with zeroes.
global modexp_bignum:
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // We store the repeated-squares accumulator x_i in scratch_1, starting with x_0 := b.
    DUP1
    // stack: length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP3
    // stack: b_start_loc, length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP8
    // stack: scratch_1, b_start_loc, length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %memcpy_kernel_general
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // We store the accumulated output value x_i in output_loc, starting with 1.
    PUSH 1
    // stack: 1, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP6
    // stack: output_loc, 1, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %mstore_kernel_general

    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
modexp_loop:
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // y := e % 2

    DUP3
    // stack: e_start_loc, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP2
    // stack: length, e_start_loc, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    ADD
    // stack: e_start_loc + length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %decrement
    // stack: e_start_loc + length - 1, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %mload_kernel_general
    // stack: e_last, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %mod_const(2)
    // stack: y = e_lst % 2 = e % 2, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    ISZERO
    // stack: y == 0, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %jumpi(modexp_y_0)

    // if y == 1, modular-multiply output_loc by scratch_1, using scratch_2..scratch_5 as scratch space, and store in scratch_6.
    PUSH modexp_mul_return
    STOP
    // stack: modexp_mul_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %stack (return, len, b, e, m, out, s1, s2, s3, s4, s5, s6) -> (len, out, s1, m, s6, s2, s3, s4, s5, return, len, b, e, m, out, s1, s2, s3, s4, s5, s6)
    // stack: length, output_loc, scratch_1, m_start_loc, scratch_6, scratch_2, scratch_3, scratch_4, scratch_5, modexp_mul_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %jump(modmul_bignum)
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

modexp_mul_return:
    // Copy scratch_6 to output_loc.
    DUP1
    // stack: length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP12
    // stack: scratch_6, length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP7
    // stack: output_loc, scratch_6, length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %memcpy_kernel_general
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // Zero out scratch_2..scratch_6.
    DUP1
    // stack: length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %mul_const(10)
    // stack: 10 * length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP8
    // stack: scratch_2, 10 * length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %clear_kernel_general
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

modexp_y_0:
    // if y == 0, do nothing

    // Modular-square repeated-squares accumulator x_i (in scratch_1), using scratch_2..scratch_5 as scratch space, and store in scratch_6.
    PUSH modexp_square_return
    // stack: modexp_square_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %stack (return, len, b, e, m, out, s1, s2, s3, s4, s5, s6) -> (len, s1, s1, m, s6, s2, s3, s4, s5, return, len, b, e, m, out, s1, s2, s3, s4, s5, s6)
    // stack: length, scratch_1, scratch_1, m_start_loc, scratch_6, scratch_2, scratch_3, scratch_4, scratch_5, modexp_square_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %jump(modmul_bignum)
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

modexp_square_return:
    // Copy scratch_6 to scratch_1.
    DUP1
    // stack: length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP12
    // stack: scratch_6, length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP8
    // stack: scratch_1, scratch_6, length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %memcpy_kernel_general
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // Zero out scratch_2..scratch_6.
    DUP1
    // stack: length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %mul_const(10)
    // stack: 10 * length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP8
    // stack: scratch_2, 10 * length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %clear_kernel_general
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // e //= 2 (with shr_bignum)

    PUSH modexp_shr_return
    // stack: modexp_shr_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP4
    // stack: e_start_loc, modexp_shr_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP3
    // stack: length, e_start_loc, modexp_shr_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %jump(shr_bignum)
modexp_shr_return:
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // check if e == 0 (with iszero_bignum)

    PUSH modexp_iszero_return
    // stack: modexp_return_6, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP4
    // stack: e_start_loc, modexp_return_6, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP3
    // stack: length, e_start_loc, modexp_return_6, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    STOP
    %jump(iszero_bignum)
modexp_iszero_return:
    // stack: e == 0, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    ISZERO
    // stack: e != 0, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %stack (iszero, vals: 4) -> (iszero, vals)
    // stack: e != 0, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %jumpi(modexp_loop)
modexp_end:
    // Copy x = x_l, in scratch_1, into output_loc

    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %stack (len, vals: 3, out, s1, ss: 5) -> (out, s1, len)
    // stack: output_loc, scratch_1, length, retdest
    %memcpy_kernel_general

    // stack: retdest
    JUMP


