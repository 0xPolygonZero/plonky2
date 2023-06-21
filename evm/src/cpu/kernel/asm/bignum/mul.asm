// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Stores a * b in output_loc, leaving a and b unchanged.
// Both a and b have length len; a * b will have length 2 * len.
// output_loc must be initialized as 2 * len zeroes.
// TODO: possible optimization: allow output_loc to be uninitialized, and write over it with a[0:len] * b[0] (a multiplication
// with carry) in place of the first addmul.
global mul_bignum:
    // stack: len, a_start_loc, b_start_loc, output_loc, retdest
    DUP1
    // stack: len, len, a_start_loc, b_start_loc, output_loc, retdest
    ISZERO
    %jumpi(len_zero)
    DUP1
    // stack: n=len, len, a_start_loc, bi=b_start_loc, output_cur=output_loc, retdest
mul_loop:
    // stack: n, len, a_start_loc, bi, output_cur, retdest
    PUSH mul_addmul_return
    // stack: mul_addmul_return, n, len, a_start_loc, bi, output_cur, retdest
    DUP5
    // stack: bi, mul_addmul_return, n, len, a_start_loc, bi, output_cur, retdest
    %mload_current_general
    // stack: b[i], mul_addmul_return, n, len, a_start_loc, bi, output_cur, retdest, b
    DUP5
    // stack: a_start_loc, b[i], mul_addmul_return, n, len, a_start_loc, bi, output_cur, retdest, b
    DUP8
    // stack: output_loc, a_start_loc, b[i], mul_addmul_return, n, len, a_start_loc, bi, output_cur, retdest, b
    DUP6
    // stack: len, output_loc, a_start_loc, b[i], mul_addmul_return, n, len, a_start_loc, bi, output_cur, retdest, b
    %jump(addmul_bignum)
mul_addmul_return:
    // stack: carry_limb, n, len, a_start_loc, bi, output_cur, retdest
    DUP6
    // stack: output_cur, carry_limb, n, len, a_start_loc, bi, output_cur, retdest
    DUP4
    // stack: len, output_cur, carry_limb, n, len, a_start_loc, bi, output_cur, retdest
    ADD
    // stack: output_cur + len, carry_limb, n, len, a_start_loc, bi, output_cur, retdest
    %mstore_current_general
    // stack: n, len, a_start_loc, bi, output_cur, retdest
    %decrement
    // stack: n-1, len, a_start_loc, bi, output_cur, retdest
    SWAP3
    %increment
    SWAP3
    // stack: n-1, len, a_start_loc, bi+1, output_cur, retdest
    SWAP4
    %increment
    SWAP4
    // stack: n-1, len, a_start_loc, bi+1, output_cur+1, retdest
    DUP1
    // stack: n-1, n-1, len, a_start_loc, bi+1, output_cur+1, retdest
    %jumpi(mul_loop)
mul_end:
    // stack: n-1, len, a_start_loc, bi+1, output_cur+1, retdest
    %pop5
    // stack: retdest
    JUMP

len_zero:
    // stack: len, a_start_loc, b_start_loc, output_loc, retdest
    %pop4
    // stack: retdest
    JUMP
