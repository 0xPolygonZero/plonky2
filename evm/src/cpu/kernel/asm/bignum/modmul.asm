// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Stores a * b % m in output_loc, leaving a, b, and m unchanged.
// a, b, and m must have the same length.
// Both output_loc and scratch_1 must have size length.
// All of scratch_2, scratch_3, and scratch_4 must have size 2 * length and be initialized with zeroes.
global modmul_bignum:
    // stack: length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, retdest
    // The prover stores x := (a * b) % m in output_loc.
    PROVER_INPUT(bignum_modmul::remainder)
    // stack: length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, retdest
    %stack (init: 4, out, scratch) -> (init, scratch, out)
    // stack: length, a_start_loc, b_start_loc, m_start_loc, scratch_1, output_loc, scratch_2, scratch_3, retdest
    // The prover stores k := (a * b) / m in scratch_1.
    PROVER_INPUT(bignum_modmul::quotient)

    // Verification step 1: calculate x + k * m.
    // Store k * m in scratch_2, using scratch_3 as scratch space.

    // Calculate a * b.
    // Store zeroes in scratch_3.
    // Store a * b in scratch_3, using scratch_4 as scratch space.

    // Check that x + k * m = a * b.
    // Walk through scratch_2 and scratch_3, checking that they are equal.
