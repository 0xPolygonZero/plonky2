// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Replaces a with a + b, leaving b unchanged.
global add_bignum_bounded:
    // stack: length, a_start_loc, b_start_loc, retdest
    %stack (l, a, b) -> (0, 0, a, b, l)
    // stack: carry=0, i=0, a_start_loc, b_start_loc, length, retdest
add_loop:
    // stack: carry, i, a_i_loc, b_i_loc, length, retdest
    DUP4
    %mload_kernel_general
    // stack: b[i], carry, i, a_i_loc, b_i_loc, length, retdest
    DUP4
    %mload_kernel_general
    // stack: a[i], b[i], carry, i, a_i_loc, b_i_loc, length, retdest
    ADD
    ADD
    // stack: a[i] + b[i] + carry, i, a_i_loc, b_i_loc, length, retdest
    %stack (val) -> (val, @BIGNUM_LIMB_BASE, @BIGNUM_LIMB_BASE, val)
    // stack: a[i] + b[i] + carry, 2^128, 2^128, a[i] + b[i] + carry, i, a_i_loc, b_i_loc, length, retdest
    DIV
    // stack: (a[i] + b[i] + carry) // 2^128, 2^128, a[i] + b[i] + carry, i, a_i_loc, b_i_loc, length, retdest
    SWAP2
    // stack: a[i] + b[i] + carry, 2^128, (a[i] + b[i] + carry) // 2^128, i, a_i_loc, b_i_loc, length, retdest
    MOD
    // stack: c[i] = (a[i] + b[i] + carry) % 2^128, carry_new = (a[i] + b[i] + carry) // 2^128, i, a_i_loc, b_i_loc, length, retdest
    DUP4
    // stack: a_i_loc, c[i], carry_new, i, a_i_loc, b_i_loc, length, retdest
    %mstore_kernel_general
    // stack: carry_new, i, a_i_loc, b_i_loc, length, retdest
    %stack (c, i, a, b) -> (a, b, c, i)
    // stack: a_i_loc, b_i_loc, carry_new, i, length, retdest
    %increment
    SWAP1
    %increment
    SWAP1
    %stack (a, b, c, i) -> (c, i, a, b)
    // stack: carry_new, i, a_i_loc + 1, b_i_loc + 1, length, retdest
    SWAP1
    %increment
    SWAP1
    // stack: carry_new, i + 1, a_i_loc + 1, b_i_loc + 1, length, retdest
    DUP5
    DUP3
    // stack: i + 1, length, carry_new, i + 1, a_i_loc + 1, b_i_loc + 1, length, retdest
    EQ
    ISZERO
    %jumpi(add_loop)
add_end:
    // stack: carry_new, i + 1, a_i_loc + 1, b_i_loc + 1, length, retdest
    %stack (c, i, a, b, n) -> (c, a)
    // stack: carry_new, a_i_loc + 1, retdest
    // If carry = 0, no need to increment.
    ISZERO
    %jumpi(increment_end)
increment_loop:
    // stack: cur_loc, retdest
    DUP1
    %mload_kernel_general
    // stack: val, cur_loc, retdest
    %increment
    // stack: val+1, cur_loc, retdest
    DUP2
    // stack: cur_loc, val+1, cur_loc, val+1, retdest
    %mstore_kernel_general
    // stack: cur_loc, val+1, retdest
    %increment
    // stack: cur_loc + 1, val+1, retdest
    SWAP1
    // stack: val+1, cur_loc + 1, retdest
    %eq_const(@BIGNUM_LIMB_BASE)
    ISZERO
    %jumpi(increment_end)
    // stack: cur_loc + 1, retdest
    PUSH 0
    DUP2
    // stack: cur_loc + 1, 0, cur_loc + 1, retdest
    %mstore_kernel_general
    %jump(increment_loop)
increment_end:
    STOP
    // cur_loc, retdest
    POP
    // retdest
    JUMP
