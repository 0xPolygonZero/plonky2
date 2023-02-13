// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Stores a * b in output_loc, leaving a and b unchanged.
// Both a and b have length len; a * b will have length 2 * len.
// Both output_loc and scratch_space must be initialized as zeroes (2 * len of them in the case
// of output_loc, and len + 1 of them in the case of scratch_space).
global mul_bignum:
    // stack: len, a_start_loc, b_start_loc, output_loc, scratch_space, retdest
    DUP1
    // stack: len, n=len, a_start_loc, bi=b_start_loc, output_cur=output_loc, scratch_space, retdest
mul_loop:
    // stack: len, n, a_start_loc, bi, output_cur, scratch_space, retdest

    // Copy a from a_start_loc into scratch_space.
    DUP1
    // stack: len, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    DUP4
    // stack: a_start_loc, len, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    DUP8
    // stack: scratch_space, a_start_loc, len, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    %memcpy_kernel_general
    // stack: len, n, a_start_loc, bi, output_cur, scratch_space, retdest

    // Insert a zero into scratch_space[len].
    DUP6
    // stack: scratch_space, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    DUP2
    // stack: len, scratch_space, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    ADD
    // stack: scratch_space + len, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    PUSH 0
    SWAP1
    // stack: scratch_space + len, 0, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    %mstore_kernel_general
    // stack: len, n, a_start_loc, bi, output_cur, scratch_space, retdest

    // Multiply a by b[i] and add into output_cur.
    PUSH mul_return
    // stack: mul_return, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    DUP5
    // stack: bi, mul_return, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    %mload_kernel_general
    // stack: b[i], mul_return, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    DUP5
    // stack: a_start_loc, b[i], mul_return, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    DUP8
    // stack: output_cur, a_start_loc, b[i], mul_return, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    DUP5
    // stack: len, output_cur, a_start_loc, b[i], mul_return, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    %jump(addmul_bignum)
mul_return:
    // stack: carry, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    DUP6
    // stack: output_cur, carry, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    DUP3
    // stack: len, output_cur, carry, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    ADD
    // stack: output_cur + len, carry, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    %increment
    // stack: output_cur + len + 1, carry, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    %mstore_kernel_general
    // stack: len, n, a_start_loc, bi, output_cur, scratch_space, retdest

    // Increment output_cur and b[i], decrement n, and check if we're done.
    DUP5
    // stack: output_cur, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    %increment
    // stack: output_cur+1, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    DUP5
    %increment
    // stack: bi+1, output_cur+1, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    DUP5
    // stack: a_start_loc, bi+1, output_cur+1, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    DUP5
    %decrement
    // stack: n-1, a_start_loc, bi+1, output_cur+1, len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    %stack (new: 4, len, old: 4) -> (len, new)
    // stack: len, n-1, a_start_loc, bi+1, output_cur+1, scratch_space, retdest
    DUP2
    // stack: n-1, len, n-1, a_start_loc, bi+1, output_cur+1, scratch_space, retdest
    %jumpi(mul_loop)
    // stack: len, n, a_start_loc, bi, output_cur, scratch_space, retdest
    %pop6
    JUMP
