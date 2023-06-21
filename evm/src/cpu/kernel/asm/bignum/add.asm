// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Adds two bignums of the same given length. Assumes that len > 0.
// Replaces a with a + b, leaving b unchanged, and returns the final carry.
global add_bignum:
    // stack: len, a_start_loc, b_start_loc, retdest
    DUP1
    ISZERO
    %jumpi(len_zero)
    // stack: len, a_start_loc, b_start_loc, retdest
    PUSH 0
    // stack: carry=0, i=len, a_cur_loc=a_start_loc, b_cur_loc=b_start_loc, retdest
add_loop:
    // stack: carry, i, a_cur_loc, b_cur_loc, retdest
    DUP4
    %mload_current_general
    // stack: b[cur], carry, i, a_cur_loc, b_cur_loc, retdest
    DUP4
    %mload_current_general
    // stack: a[cur], b[cur], carry, i, a_cur_loc, b_cur_loc, retdest
    ADD
    ADD
    // stack: a[cur] + b[cur] + carry, i, a_cur_loc, b_cur_loc, retdest
    DUP1
    // stack: a[cur] + b[cur] + carry, a[cur] + b[cur] + carry, i, a_cur_loc, b_cur_loc, retdest
    %shr_const(128)
    // stack: (a[cur] + b[cur] + carry) // 2^128, a[cur] + b[cur] + carry, i, a_cur_loc, b_cur_loc, retdest
    SWAP1
    // stack: a[cur] + b[cur] + carry, (a[cur] + b[cur] + carry) // 2^128, i, a_cur_loc, b_cur_loc, retdest
    %mod_const(0x100000000000000000000000000000000)
    // stack: c[cur] = (a[cur] + b[cur] + carry) % 2^128, carry_new = (a[cur] + b[cur] + carry) // 2^128, i, a_cur_loc, b_cur_loc, retdest
    DUP4
    // stack: a_cur_loc, c[cur], carry_new, i, a_cur_loc, b_cur_loc, retdest
    %mstore_current_general
    // stack: carry_new, i, a_cur_loc, b_cur_loc, retdest
    SWAP2
    %increment
    SWAP2
    // stack: carry_new, i, a_cur_loc + 1, b_cur_loc, retdest
    SWAP3
    %increment
    SWAP3
    // stack: carry_new, i, a_cur_loc + 1, b_cur_loc + 1, retdest
    SWAP1
    %decrement
    SWAP1
    // stack: carry_new, i - 1, a_cur_loc + 1, b_cur_loc + 1, retdest
    DUP2
    // stack: i - 1, carry_new, i - 1, a_cur_loc + 1, b_cur_loc + 1, retdest
    %jumpi(add_loop)
add_end:
    // stack: carry_new, i - 1, a_cur_loc + 1, b_cur_loc + 1, retdest
    %stack (c, i, a, b) -> (c)
    // stack: carry_new, retdest
    SWAP1
    // stack: retdest, carry_new
    JUMP

len_zero:
    // stack: len, a_start_loc, b_start_loc, retdest
    %pop3
    // stack: retdest
    PUSH 0
    // stack: carry=0, retdest
    SWAP1
    JUMP
