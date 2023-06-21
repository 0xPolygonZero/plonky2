// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Stores a * b % m in output_loc, leaving a, b, and m unchanged.
// a, b, and m must have the same length.
// output_loc must have size length; scratch_2 must have size 2*length.
// Both scratch_2 and scratch_3 have size 2*length and be initialized with zeroes.

// The prover provides x := (a * b) % m, which is the output of this function.
// We first check that x < m.
// The prover also provides k := (a * b) / m, stored in scratch space.
// We then check that x + k * m = a * b, by computing both of those using
// bignum arithmetic, storing the results in scratch space.
// We assert equality between those two, limb by limb.
global modmul_bignum:
    // stack: len, a_loc, b_loc, m_loc, out_loc, s1 (=scratch_1), s2, s3, retdest
    DUP1
    ISZERO
    %jumpi(len_zero)
    
    // STEP 1:
    // The prover provides x := (a * b) % m, which we store in output_loc.
    
    PUSH 0
    // stack: i=0, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
modmul_remainder_loop:
    // stack: i, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    PROVER_INPUT(bignum_modmul)
    // stack: PI, i, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    DUP7
    DUP3
    ADD
    // stack: out_loc[i], PI, i, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    %mstore_current_general
    // stack: i, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    %increment
    DUP2
    DUP2
    // stack: i+1, len, i+1, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    SUB // functions as NEQ
    // stack: i+1!=len, i+1, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    %jumpi(modmul_remainder_loop)
// end of modmul_remainder_loop
    // stack: i, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    POP

    // stack: len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest

    // STEP 2:
    // We check that x < m.

    PUSH modmul_return_1
    DUP6
    DUP6
    DUP4
    // stack: len, m_loc, out_loc, modmul_return_1, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    // Should return 1 iff the value at m_loc > the value at out_loc; in other words, if x < m.
    %jump(cmp_bignum)
modmul_return_1:
    // stack: cmp_result, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    PUSH 1
    %assert_eq

    // STEP 3:
    // The prover provides k := (a * b) / m, which we store in scratch_1.

    // stack: len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    DUP1
    // stack: len, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    %mul_const(2)
    // stack: 2*len, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    PUSH 0
    // stack: i=0, 2*len, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
modmul_quotient_loop:
    // stack: i, 2*len, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    PROVER_INPUT(bignum_modmul)
    // stack: PI, i, 2*len, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    DUP9
    DUP3
    ADD
    // stack: s1[i], PI, i, 2*len, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    %mstore_current_general
    // stack: i, 2*len, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    %increment
    DUP2
    DUP2
    // stack: i+1, 2*len, i+1, 2*len, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    SUB // functions as NEQ
    // stack: i+1!=2*len, i+1, 2*len, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    %jumpi(modmul_quotient_loop)
// end of modmul_quotient_loop
    // stack: i, 2*len, len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    %pop2
    // stack: len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest

    // STEP 4:
    // We calculate x + k * m.

    // STEP 4.1:
    // Multiply k with m and store k * m in scratch_2.
    PUSH modmul_return_2
    %stack (return, len, a, b, m, out, s1, s2) -> (len, s1, m, s2, return, len, a, b, out, s2)
    // stack: len, s1, m_loc, s2, modmul_return_2, len, a_loc, b_loc, out_loc, s2, s3, retdest
    %jump(mul_bignum)
modmul_return_2:
    // stack: len, a_loc, b_loc, out_loc, s2, s3, retdest

    // STEP 4.2:
    // Add x into k * m (in scratch_2).
    PUSH modmul_return_3
    %stack (return, len, a, b, out, s2) -> (len, s2, out, return, len, a, b, s2)
    // stack: len, s2, out_loc, modmul_return_3, len, a_loc, b_loc, s2, s3, retdest
    %jump(add_bignum)
modmul_return_3:
    // stack: carry, len, a_loc, b_loc, s2, s3, retdest
    POP
    // stack: len, a_loc, b_loc, s2, s3, retdest

    // STEP 5:
    // We calculate a * b.

    // Multiply a with b and store a * b in scratch_3.
    PUSH modmul_return_4
    %stack (return, len, a, b, s2, s3) -> (len, a, b, s3, return, len, s2, s3)
    // stack: len, a_loc, b_loc, s3, modmul_return_4, len, s2, s3, retdest
    %jump(mul_bignum)
modmul_return_4:
    // stack: len, s2, s3, retdest

    // STEP 6:
    // Check that x + k * m = a * b.

    // Walk through scratch_2 and scratch_3, checking that they are equal.
    // stack: n=len, i=s2, j=s3, retdest
modmul_check_loop:
    // stack: n, i, j, retdest
    %stack (l, idx: 2) -> (idx, l, idx)
    // stack: i, j, n, i, j, retdest
    %mload_current_general
    SWAP1
    %mload_current_general
    SWAP1
    // stack: mem[i], mem[j], n, i, j, retdest
    %assert_eq
    // stack: n, i, j, retdest
    %decrement
    SWAP1
    %increment
    SWAP2
    %increment
    SWAP2
    SWAP1
    // stack: n-1, i+1, j+1, retdest
    DUP1
    // stack: n-1, n-1, i+1, j+1, retdest
    %jumpi(modmul_check_loop)
// end of modmul_check_loop
    // stack: n-1, i+1, j+1, retdest
    %pop3
    // stack: retdest
    JUMP

len_zero:
    // stack: len, a_loc, b_loc, m_loc, out_loc, s1, s2, s3, retdest
    %pop8
    // stack: retdest
    JUMP
