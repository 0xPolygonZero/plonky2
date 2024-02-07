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
    %build_current_general_address_no_offset
    PUSH 0
    // stack: carry=0, base_addr, i=len, a_cur_loc=a_start_loc, b_cur_loc=b_start_loc, retdest
add_loop:
    // stack: carry, base_addr, i, a_cur_loc, b_cur_loc, retdest
    DUP2
    // stack: base_addr, carry, base_addr, i, a_cur_loc, b_cur_loc, retdest
    DUP6 ADD // base_addr + b_cur_loc
    MLOAD_GENERAL
    // stack: b[cur], carry, base_addr, i, a_cur_loc, b_cur_loc, retdest
    DUP3
    DUP6 ADD // base_addr + a_cur_loc
    MLOAD_GENERAL
    // stack: a[cur], b[cur], carry, base_addr, i, a_cur_loc, b_cur_loc, retdest
    ADD
    ADD
    // stack: a[cur] + b[cur] + carry, base_addr, i, a_cur_loc, b_cur_loc, retdest
    DUP1
    // stack: a[cur] + b[cur] + carry, a[cur] + b[cur] + carry, base_addr, i, a_cur_loc, b_cur_loc, retdest
    %shr_const(128)
    // stack: (a[cur] + b[cur] + carry) // 2^128, a[cur] + b[cur] + carry, base_addr, i, a_cur_loc, b_cur_loc, retdest
    SWAP1
    // stack: a[cur] + b[cur] + carry, (a[cur] + b[cur] + carry) // 2^128, base_addr, i, a_cur_loc, b_cur_loc, retdest
    %mod_const(0x100000000000000000000000000000000)
    // stack: c[cur] = (a[cur] + b[cur] + carry) % 2^128, carry_new = (a[cur] + b[cur] + carry) // 2^128, base_addr, i, a_cur_loc, b_cur_loc, retdest
    DUP3
    DUP6
    ADD // base_addr + a_cur_loc
    // stack: a_cur_addr, c[cur], carry_new,  base_addr, i, a_cur_loc, b_cur_loc, retdest
    %swap_mstore
    // stack: carry_new, base_addr, i, a_cur_loc, b_cur_loc, retdest
    SWAP3
    %increment
    SWAP3
    // stack: carry_new, base_addr, i, a_cur_loc + 1, b_cur_loc, retdest
    SWAP4
    %increment
    SWAP4
    // stack: carry_new, base_addr, i, a_cur_loc + 1, b_cur_loc + 1, retdest
    SWAP2
    %decrement
    SWAP2
    // stack: carry_new, base_addr, i - 1, a_cur_loc + 1, b_cur_loc + 1, retdest
    DUP3
    // stack: i - 1, carry_new, base_addr, i - 1, a_cur_loc + 1, b_cur_loc + 1, retdest
    %jumpi(add_loop)
add_end:
    // stack: carry_new, base_addr, i - 1, a_cur_loc + 1, b_cur_loc + 1, retdest
    %stack (c, addr, i, a, b) -> (c)
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
