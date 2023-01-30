// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Stores b ^ e % m in output_loc, leaving b, e, and m unchanged.
// b, e, and m must have the same length.
// Both output_loc and scratch_1 must have size length.
// All of scratch_2, scratch_3, and scratch_4 must have size 2 * length and be initialized with zeroes.
global modexp_bignum:
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    PUSH 0
    // stack: i = 0, length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest

    // We store x_i in scratch_1, starting with x_0 := b.


    // Loop: while e is not zero:

    // y := e % 2

    // Verifier's goal: check that x_(i+1) + k_i * m = x_i^2 * b^y.

    // Prover supplies k_i = x_i^2 * b^y // m into scratch_2.

    // Multiply k_i (in scratch_2) by m and store in scratch_3.

    // Prover supplies x_(i+1) = x_i^2 * b^y % m into scratch_2.

    // Add x_(i+1) (in scratch_2) into k_i * m (in scratch_3).

    // Multiply x_i (in scratch_1) by x_i (in scratch_1) and store in scratch_2.

    // If y == 1, multiply x_i^2 (in scratch_2) by b and store in scratch_4.

    // If y == 0, just copy x_i^2 (in scratch_2) into scratch_4.

    // Check that x_(i+1) + k_i * m (in scratch_3) = x_i^2 * b^y (in scratch_4).

    // Update for next round: 

    // e //= 2 (with shr_bignum)

    // i += 1

    // check if e == 0 (with iszero_bignum)


    // At end:
    // write x = x_l into output_loc

