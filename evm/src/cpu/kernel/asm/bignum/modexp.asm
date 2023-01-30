// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Stores b ^ e % m in output_loc, leaving b, e, and m unchanged.
// b, e, and m must have the same length.
// Both output_loc and scratch_1 must have size length.
// All of scratch_2, scratch_3, and scratch_4 must have size 2 * length and be initialized with zeroes.
global modexp_bignum:
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest

    // We store x_i in scratch_1, starting with x_0 := b.
    DUP1
    // stack: length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP3
    // stack: b_start_loc, length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP8
    // stack: scratch_1, b_start_loc, length, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %memcpy_kernel_general
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest

    PUSH 0
    // stack: i=0, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
modexp_loop:
    // stack: i, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest

    // y := e % 2

    DUP4
    // stack: e_start_loc, i, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    DUP3
    // stack: length, e_start_loc, i, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    ADD
    // stack: e_start_loc + length, i, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %decrement
    // stack: e_start_loc + length - 1, i, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %mload_kernel_general
    // stack: e_last, i, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest
    %mod_const(2)
    // stack: y, i, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, retdest

    

    // Verifier's goal: check that x_(i+1) + k_i * m = x_i^2 * b^y.

    // Prover supplies k_i = x_i^2 * b^y // m into scratch_2.

    // Multiply k_i (in scratch_2) by m and store in scratch_3.

    // Prover supplies x_(i+1) = x_i^2 * b^y % m into scratch_2.

    // Add x_(i+1) (in scratch_2) into k_i * m (in scratch_3).

    // Multiply x_i (in scratch_1) by x_i (in scratch_1) and store in scratch_4.

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

