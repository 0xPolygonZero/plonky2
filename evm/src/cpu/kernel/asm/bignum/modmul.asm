// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Stores a * b % m in output_loc, leaving a, b, and m unchanged.
global modmul_bignum:
    // stack: length, a_start_loc, b_start_loc, m_start_loc, output_loc, retdest
    