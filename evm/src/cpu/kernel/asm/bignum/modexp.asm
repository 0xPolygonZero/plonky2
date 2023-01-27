// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Stores b ^ e % m in output_loc, leaving b, e, and m unchanged.
// b, e, and m must have the same length.
// Both output_loc and scratch_1 must have size length.
// All of scratch_2, scratch_3, and scratch_4 must have size 2 * length and be initialized with zeroes.
global modexp_bignum:
    // stack: length, b_start_loc, e_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    

    // We store x_i in scratch_1, starting with x_0 := b.


    // Loop: while e is not zero:

    // y := e % 2

    // Prover supplies x_(i+1) = x_i^2 * b^y % m into scratch_2

    // Prover supplies k_i = x_i^2 * b^y // m into scratch_3

    // Verifier checks that x_(i+1) + k_i * m = x_i^2 * b^y

