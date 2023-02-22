// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Stores b ^ e % m in output_loc, leaving b, e, and m unchanged.
// b, e, and m must have the same length.
// output_loc must have size length and be initialized with zeroes; scratch_1 must have size length.
// All of scratch_2..scratch_5 must have size 2 * length and be initialized with zeroes.
global modexp_bignum:
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest

    // We store the repeated-squares accumulator x_i in scratch_1, starting with x_0 := b.
    DUP1
    // stack: length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP3
    // stack: b_start_loc, length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP8
    // stack: scratch_1, b_start_loc, length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %memcpy_kernel_general
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest

    // We store the accumulated output value x_i in output_loc, starting with x_0=1.
    PUSH 1
    // stack: 1, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP6
    // stack: output_loc, 1, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5,  retdest
    %mstore_kernel_general

modexp_loop:
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest

    // y := e % 2
    DUP3
    // stack: e_start_loc, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %mload_kernel_general
    // stack: e_first, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %mod_const(2)
    // stack: y = e_first % 2 = e % 2, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    ISZERO
    // stack: y == 0, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %jumpi(modexp_y_0)

    // if y == 1, modular-multiply output_loc by scratch_1, using scratch_2..scratch_4 as scratch space, and store in scratch_5.
    PUSH modexp_mul_return
    // stack: modexp_mul_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP10
    // stack: scratch_4, modexp_mul_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP10
    // stack: scratch_3, scratch_4, modexp_mul_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP10
    // stack: scratch_2, scratch_3, scratch_4, modexp_mul_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP14
    // stack: scratch_5, scratch_2, scratch_3, scratch_4, modexp_mul_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP9
    // stack: m_start_loc, scratch_5, scratch_2, scratch_3, scratch_4, modexp_mul_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP12
    // stack: scratch_1, m_start_loc, scratch_5, scratch_2, scratch_3, scratch_4, modexp_mul_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP12
    // stack: output_loc, scratch_1, m_start_loc, scratch_5, scratch_2, scratch_3, scratch_4, modexp_mul_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP9
    // stack: length, output_loc, scratch_1, m_start_loc, scratch_5, scratch_2, scratch_3, scratch_4, modexp_mul_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %jump(modmul_bignum)
modexp_mul_return:
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest

    // Copy scratch_5 to output_loc.
    DUP1
    // stack: length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP11
    // stack: scratch_5, length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP7
    // stack: output_loc, scratch_5, length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %memcpy_kernel_general
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest

    // Zero out scratch_2..scratch_5.
    DUP1
    // stack: length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %mul_const(8)
    // stack: 8 * length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP8
    // stack: scratch_2, 8 * length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %clear_kernel_general
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest

modexp_y_0:
    // if y == 0, do nothing

    // Modular-square repeated-squares accumulator x_i (in scratch_1), using scratch_2..scratch_4 as scratch space, and store in scratch_5.
    PUSH modexp_square_return
    // stack: modexp_square_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP10
    // stack: scratch_4, modexp_square_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP10
    // stack: scratch_3, scratch_4, modexp_square_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP10
    // stack: scratch_2, scratch_3, scratch_4, modexp_square_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP14
    // stack: scratch_5, scratch_2, scratch_3, scratch_4, modexp_square_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP9
    // stack: m_start_loc, scratch_5, scratch_2, scratch_3, scratch_4, modexp_square_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP12
    // stack: scratch_1, m_start_loc, scratch_5, scratch_2, scratch_3, scratch_4, modexp_square_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP1
    // stack: scratch_1, scratch_1, m_start_loc, scratch_5, scratch_2, scratch_3, scratch_4, modexp_square_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP9
    // stack: length, scratch_1, scratch_1, m_start_loc, scratch_5, scratch_2, scratch_3, scratch_4, modexp_square_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %jump(modmul_bignum)
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest

modexp_square_return:
    // Copy scratch_5 to scratch_1.
    DUP1
    // stack: length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP11
    // stack: scratch_5, length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP8
    // stack: scratch_1, scratch_5, length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %memcpy_kernel_general
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest

    // Zero out scratch_2..scratch_5.
    DUP1
    // stack: length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %mul_const(8)
    // stack: 8 * length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP8
    // stack: scratch_2, 8 * length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %clear_kernel_general
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest

    // e //= 2 (with shr_bignum)

    PUSH modexp_shr_return
    // stack: modexp_shr_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP4
    // stack: e_start_loc, modexp_shr_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP3
    // stack: length, e_start_loc, modexp_shr_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %jump(shr_bignum)
modexp_shr_return:
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest

    // check if e == 0 (with iszero_bignum)
    PUSH modexp_iszero_return
    // stack: modexp_iszero_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP4
    // stack: e_start_loc, modexp_iszero_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP3
    // stack: length, e_start_loc, modexp_iszero_return, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %jump(iszero_bignum)
modexp_iszero_return:
    // stack: e == 0, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    ISZERO
    // stack: e != 0, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %jumpi(modexp_loop)
modexp_end:
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %rep 10
        POP
    %endrep
    // stack: retdest
    JUMP


