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
    %neq
    // stack: j+1 != length, length, j+1, length, scratch_1, b_start_loc, y, m_start_loc, e_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %jumpi(modexp_quotient_loop)
modexp_quotient_end:
    // stack: j, length, scratch_1, b_start_loc, y, m_start_loc, e_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    POP
    // stack: length, scratch_1, b_start_loc, y, m_start_loc, e_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // Multiply k_i (in scratch_2) by m and store in scratch_3, using scratch_4 as scratch space.
    PUSH modexp_return_1
    // stack: modexp_return_1, length, scratch_1, b_start_loc, y, m_start_loc, e_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %stack (return, len, s1, b, y, m, e, i, out, s11, s2, s3, s4) -> (len, s2, m, s3, s4, return, len, b, e, m, i, y, out, s1, s2, s3, s4)
    // stack: length, scratch_2, m_start_loc, scratch_3, scratch_4, modexp_return_1, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %jump(mul_bignum)
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
    %neq
    // stack: j+1 != length, j+1, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %jumpi(modexp_remainder_loop)
modexp_remainder_end:
    // stack: j, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    POP
    // stack: length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // Add x_(i+1) (in scratch_2) into k_i * m (in scratch_3).
    PUSH modexp_return_2
    // stack: modexp_return_2, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %stack (return, len, others: 7, s2, s3) -> (len, s3, s2, return, len, others, s2, s3)
    // stack: length, scratch_3, scratch_2, modexp_return_2, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
modexp_return_2:
    // stack: length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // Multiply x_i (in scratch_1) by x_i (in scratch_1) and store in scratch_4, using scratch_5 as scratch space.
    PUSH modexp_return_3
    // stack: modexp_return_3, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %stack (return, len, others: 6, s1, s2, s3, s4, s5) -> (len, s1, s1, s4, s5, return, len, others, s1, s2, s3, s4, s5)
    // stack: length, scratch_1, scratch_1, scratch_4, scratch_5, modexp_return_3, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %jump(mul_bignum)
modexp_return_3:
    // stack: length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // Zero out scratch_5.
    DUP1
    // stack: length, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP13
    // stack: scratch_5, length, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %clear_kernel_general
    // stack: length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP7
    // stack: y, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    ISZERO
    // stack: y == 0, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %jumpi(modexp_case_y_0)
modexp_case_y_1:
    // If y == 1, multiply x_i^2 (in scratch_4) by b and store in scratch_5 (using scratch_6 as scratch space).
    PUSH modexp_return_4
    // stack: modexp_return_4, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %stack (return, len, b, e, m, i, y, out, ss: 3, s4, s5, s6) -> (len, s4, b, s5, s6, return, len, b, e, m, i, out, ss, s4, s5, s6)
    // stack: length, scratch_4, b_start_loc, scratch_5, scratch_6, modexp_return_4, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %jump(mul_bignum)
modexp_case_y_0:
    // If y == 0, just copy x_i^2 (in scratch_4) into scratch_5.
    // stack: length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP1
    // stack: length, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP14
    // stack: scratch_4, length, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP16
    // stack: scratch_5, scratch_4, length, length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %memcpy_kernel_general
    // stack: length, b_start_loc, e_start_loc, m_start_loc, i, y, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %stack (start: 5, y) -> (start)
    // stack: length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
modexp_return_4:
    // stack: length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // Verification: check that x_(i+1) + k_i * m (in scratch_3) = x_i^2 * b^y (in scratch_5).
    // Walk through scratch_3 and scratch_5, and check that they are equal.
    DUP11
    // stack: scratch_5, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP10
    // stack: scratch_3, scratch_5, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP3
    // stack: n=length, a=scratch_3, b=scratch_5, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
modexp_check_loop:
    // stack: n, a, b, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %stack (l, idx: 2) -> (idx, l, idx)
    // stack: a, b, n, a, b, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %mload_kernel_general
    SWAP1
    %mload_kernel_general
    SWAP1
    // stack: mem[a], mem[b], n, a, b, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %assert_eq
    // stack: n, a, b, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %decrement
    // stack: n-1, a, b, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    SWAP1
    // stack: a, n-1, b, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %increment
    // stack: a+1, n-1, b, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    SWAP2
    // stack: b, n-1, a+1, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %increment
    // stack: b+1, n-1, a+1, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    SWAP2
    SWAP1
    // stack: n-1, a+1, b+1, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP1
    // stack: n-1, n-1, a+1, b+1, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %jumpi(modexp_check_loop)
modexp_check_end:
    // stack: n-1, a+1, b+1, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %pop3
    // stack: length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // Update for next round: 

    // Copy x_(i+1) (in scratch_2) into x_i (in scratch_1).
    DUP1
    // stack: length, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP9
    // stack: scratch_2, length, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP9
    // stack: scratch_1, scratch_2, length, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %memcpy_kernel_general

    // stack: length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // Zero out scratch_3, scratch_4, scratch_5, and scratch_6.
    DUP1
    DUP10
    %clear_kernel_general
    DUP1
    DUP11
    %clear_kernel_general
    DUP1
    DUP12
    %clear_kernel_general
    DUP1
    DUP13
    %clear_kernel_general

    // e //= 2 (with shr_bignum)

    PUSH modexp_return_5
    // stack: modexp_return_5, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP4
    // stack: e_start_loc, modexp_return_5, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP3
    // stack: length, e_start_loc, modexp_return_5, length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %jump(shr_bignum)
modexp_return_5:
    // stack: length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    
    // i += 1
    SWAP4
    %increment
    SWAP4

    // stack: length, b_start_loc, e_start_loc, m_start_loc, i + 1, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest

    // check if e == 0 (with iszero_bignum)

    PUSH modexp_return_6
    // stack: modexp_return_6, length, b_start_loc, e_start_loc, m_start_loc, i + 1, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP4
    // stack: e_start_loc, modexp_return_6, length, b_start_loc, e_start_loc, m_start_loc, i + 1, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    DUP3
    // stack: length, e_start_loc, modexp_return_6, length, b_start_loc, e_start_loc, m_start_loc, i + 1, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %jump(iszero_bignum)
modexp_return_6:
    // stack: e == 0, length, b_start_loc, e_start_loc, m_start_loc, i + 1, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    ISZERO
    // stack: e != 0, length, b_start_loc, e_start_loc, m_start_loc, i + 1, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %jumpi(modexp_loop)
modexp_end:
    // Copy x = x_l, in scratch_1, into output_loc

    // stack: length, b_start_loc, e_start_loc, m_start_loc, i, output_loc, scratch_1, scratch_2, scratch_3, scratch_4, scratch_5, scratch_6, retdest
    %stack (len, vals: 4, out, s1, ss: 5) -> (out, s1, len)
    // stack: output_loc, scratch_1, length, retdest
    %memcpy_kernel_general
    // stack: retdest
    JUMP


