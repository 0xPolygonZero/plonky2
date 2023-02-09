// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Stores a * b % m in output_loc, leaving a, b, and m unchanged.
// a, b, and m must have the same length.
// Both output_loc and scratch_1 must have size length.
// All of scratch_2, scratch_3, and scratch_4 must have size 2 * length and be initialized with zeroes.
global modmul_bignum:
    // stack: length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    
    // The prover provides x := (a * b) % m, which we store in output_loc.
    
    PUSH 0
    // stack: i=0, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
modmul_remainder_loop:
    // stack: i, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    PROVER_INPUT(bignum_modmul::remainder)
    // stack: PI, i, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    DUP7
    // stack: output_loc, PI, i, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    DUP3
    // stack: i, output_loc, PI, i, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    ADD
    // stack: output_loc[i], PI, i, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    %mstore_kernel_general
    // stack: i, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    %increment
    // stack: i+1, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    DUP2
    DUP2
    // stack: i+1, length, i+1, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    EQ
    // stack: i+1==length, i+1, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    ISZERO
    // stack: i+1!=length, i+1, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    %jumpi(modmul_remainder_loop)
modmul_remainder_end:
    // stack: i, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    POP

    // The prover provides k := (a * b) / m, which we store in scratch_1.

    // stack: length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    PUSH 0
    // stack: i=0, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
modmul_quotient_loop:
    // stack: i, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    PROVER_INPUT(bignum_modmul::quotient)
    // stack: PI, i, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    DUP8
    // stack: scratch_1, PI, i, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    DUP3
    // stack: i, scratch_1, PI, i, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    ADD
    // stack: scratch_1[i], PI, i, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    %mstore_kernel_general
    // stack: i, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    %increment
    // stack: i+1, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    DUP2
    DUP2
    // stack: i+1, length, i+1, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    %neq
    // stack: i+1!=length, i+1, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    %jumpi(modmul_quotient_loop)
modmul_quotient_end:
    // stack: i, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    POP
    // stack: length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest

    // Verification step 1: calculate x + k * m.

    // Store k * m in scratch_2, using scratch_3 as scratch space.
    PUSH modmul_return_1
    // stack: modmul_return_1, length, a_start_loc, b_start_loc, m_start_loc, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, retdest
    %stack (return, len, a, b, m, out, s1, s2, s3) -> (len, s1, m, s2, s3, return, len, a, b, out, s2, s3)
    // stack: length, scratch_1, m_start_loc, scratch_2, scratch_3, modmul_return_1, length, a_start_loc, b_start_loc, output_loc, scratch_2, scratch_3, scratch_4, retdest
    %jump(mul_bignum)
modmul_return_1:
    // stack: length, a_start_loc, b_start_loc, output_loc, scratch_2, scratch_3, scratch_4, retdest

    // Add x into k * m (in scratch_2).
    PUSH modmul_return_2
    // stack: modmul_return_2, length, a_start_loc, b_start_loc, output_loc, scratch_2, scratch_3, scratch_4, retdest
    %stack (return, len, a, b, out, s2) -> (len, s2, out, return, len, a, b, s2)
    // stack: length, scratch_2, output_loc, modmul_return_2, length, a_start_loc, b_start_loc, scratch_2, scratch_3, scratch_4, retdest
    %jump(add_bignum)
modmul_return_2:
    // stack: length, a_start_loc, b_start_loc, scratch_2, scratch_3, scratch_4, retdest

    // Calculate a * b.

    // Store zeroes in scratch_3.
    DUP1
    // stack: length, length, a_start_loc, b_start_loc, scratch_2, scratch_3, scratch_4, retdest
    DUP6
    // stack: scratch_3, length, length, a_start_loc, b_start_loc, scratch_2, scratch_3, scratch_4, retdest
    %clear_kernel_general
    // stack: length, a_start_loc, b_start_loc, scratch_2, scratch_3, scratch_4, retdest

    // Store a * b in scratch_3, using scratch_4 as scratch space.
    PUSH modmul_return_3
    // stack: modmul_return_3, length, a_start_loc, b_start_loc, scratch_2, scratch_3, scratch_4, retdest
    %stack (return, len, a, b, s2, s3, s4) -> (len, a, b, s3, s4, return, len, s2, s3)
    // stack: length, a_start_loc, b_start_loc, scratch_3, scratch_4, modmul_return_3, length, scratch_2, scratch_3, retdest
    %jump(mul_bignum)
modmul_return_3:
    // stack: length, scratch_2, scratch_3, retdest

    // Check that x + k * m = a * b.
    // Walk through scratch_2 and scratch_3, checking that they are equal.
    // stack: n=length, i=scratch_2, j=scratch_3, retdest
modmul_check_loop:
    // stack: n, i, j, retdest
    %stack (l, idx: 2) -> (idx, l, idx)
    // stack: i, j, n, i, j, retdest
    %mload_kernel_general
    SWAP1
    %mload_kernel_general
    SWAP1
    // stack: mem[i], mem[j], n, i, j, retdest
    %assert_eq
    // stack: n, i, j, retdest
    %decrement
    // stack: n-1, i, j, retdest
    SWAP1
    // stack: i, n-1, j, retdest
    %increment
    // stack: i+1, n-1, j, retdest
    SWAP2
    // stack: j, n-1, i+1, retdest
    %increment
    // stack: j+1, n-1, i+1, retdest
    SWAP2
    SWAP1
    // stack: n-1, i+1, j+1, retdest
    DUP1
    // stack: n-1, n-1, i+1, j+1, retdest
    %jumpi(modmul_check_loop)
modmul_check_end:
    // stack: n-1, i+1, j+1, retdest
    %pop3
    // stack: retdest
    JUMP
