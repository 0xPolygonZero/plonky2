// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Stores b ^ e % m in output_loc, leaving b, e, and m unchanged.
// b, e, and m must have the same length.
// Both output_loc and scratch_1 must have size length.
// All of scratch_2, scratch_3, and scratch_4 must have size 2 * length and be initialized with zeroes.
global modexp_bignum:
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // We store x_i in scratch_1, starting with x_0 := b.
    DUP1
    // stack: length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP3
    // stack: b_start_loc, length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP8
    // stack: scratch_1, b_start_loc, length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %memcpy_kernel_general
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    PUSH 0
    // stack: i=0, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
modexp_loop:
    // stack: i, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // y := e % 2

    DUP4
    // stack: e_start_loc, i, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP3
    // stack: length, e_start_loc, i, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    ADD
    // stack: e_start_loc + length, i, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %decrement
    // stack: e_start_loc + length - 1, i, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %mload_kernel_general
    // stack: e_last, i, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %mod_const(2)
    // stack: y = e_lst % 2 = e % 2, i, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // Verifier's goal: check that x_(i+1) + k_i * m = x_i^2 * b^y.

    // Prover supplies k_i = x_i^2 * b^y // m into scratch_2.

    // stack: y, i, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP8
    // stack: scratch_1, y, i, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %stack (s1, y, i, len, b, e, m) -> (len, s1, b, y, m, e, i)
    // stack: length, scratch_1, b_start_loc, y, m_start_loc, e_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    PUSH 0
    // stack: j=0, length, scratch_1, b_start_loc, y, m_start_loc, e_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
modexp_quotient_loop:
    // stack: j, length, scratch_1, b_start_loc, y, m_start_loc, e_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    PROVER_INPUT(bignum_modexp::quotient)
    // stack: PI, j, length, scratch_1, b_start_loc, y, m_start_loc, e_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP12
    // stack: scratch_2, PI, j, length, scratch_1, b_start_loc, y, m_start_loc, e_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP3
    // stack: j, scratch_2, PI, j, length, scratch_1, b_start_loc, y, m_start_loc, e_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    ADD
    // stack: scratch_2[j], PI, j, length, scratch_1, b_start_loc, y, m_start_loc, e_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %mstore_kernel_general
    // stack: j, length, scratch_1, b_start_loc, y, m_start_loc, e_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %increment
    // stack: j+1, length, scratch_1, b_start_loc, y, m_start_loc, e_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP2
    DUP2
    // stack: j+1, length, j+1, length, scratch_1, b_start_loc, y, m_start_loc, e_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    NE
    // stack: j+1 != length, length, j+1, length, scratch_1, b_start_loc, y, m_start_loc, e_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %jumpi(modexp_quotient_loop)
modexp_quotient_end:
    // stack: j, length, scratch_1, b_start_loc, y, m_start_loc, e_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    POP
    // stack: length, scratch_1, b_start_loc, y, m_start_loc, e_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // Multiply k_i (in scratch_2) by m and store in scratch_3, using scratch_4 as scratch space.
    PUSH modexp_return_1
    // stack: modexp_return_1, length, scratch_1, b_start_loc, y, m_start_loc, e_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %stack (return, len, s1, b, y, m, e, i, out, s11, s2, s3, s4) -> (len, s2, m, s3, s4, return, len, b, e, m, i, y out, s1, s2, s3, s4)
    // stack: length, scratch_2, m_start_loc, scratch_3, scratch_4, modexp_return_1, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %jump(modmul_bignum)
modexp_return_1:
    // stack: length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // Zero out scratch_4.
    DUP1
    // stack: length, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP12
    // stack: scratch_4, length, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %clear_kernel_general
    // stack: length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // Prover supplies x_(i+1) = x_i^2 * b^y % m into scratch_2.

    // stack: length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    PUSH 0
    // stack: j=0, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
modexp_remainder_loop:
    // stack: j, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    PROVER_INPUT(bignum_modexp::remainder)
    // stack: PI, j, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP11
    // stack: scratch_2, PI, j, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP3
    // stack: j, scratch_2, PI, j, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    ADD
    // stack: scratch_2[j], PI, j, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %mstore_kernel_general
    // stack: j, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %increment
    // stack: j+1, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP2
    DUP2
    // stack: j+1, length, j+1, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    NE
    // stack: j+1 != length, j+1, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %jumpi(modexp_remainder_loop)
modexp_remainder_end:

    // Add x_(i+1) (in scratch_2) into k_i * m (in scratch_3).

    // Multiply x_i (in scratch_1) by x_i (in scratch_1) and store in scratch_4, using scratch_5 as scratch space.

    // Zero out scratch_5.

    // If y == 1, multiply x_i^2 (in scratch_4) by b and store in scratch_5.

    // If y == 0, just copy x_i^2 (in scratch_4) into scratch_5.

    // Check that x_(i+1) + k_i * m (in scratch_3) = x_i^2 * b^y (in scratch_5).

    // Update for next round: 

    // Copy x_(i+1) (in scratch_2) into x_i (in scratch_1).

    // e //= 2 (with shr_bignum)

    // i += 1

    // check if e == 0 (with iszero_bignum)


modexp_end:
    // write x = x_l into output_loc

