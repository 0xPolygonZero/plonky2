// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Return a >= b.
global ge_bignum_bounded:
    // stack: length, a_start_loc, b_start_loc, retdest
    SWAP1
    // stack: a_start_loc, length, b_start_loc, retdest
    DUP2
    // stack: length, a_start_loc, length, b_start_loc, retdest
    ADD
    %decrement
    // stack: a_end_loc, length, b_start_loc, retdest
    SWAP2
    // stack: b_start_loc, length, a_end_loc, retdest
    DUP2
    // stack: length, b_start_loc, length, a_end_loc, retdest
    ADD
    %decrement
    // stack: b_end_loc, length, a_end_loc, retdest
    %stack (b, l, a) -> (l, a, b)
    // stack: length, a_end_loc, b_end_loc, retdest
    %decrement
ge_loop:
    // stack: i, a_i_loc, b_i_loc, retdest
    DUP3
    DUP3
    // stack: a_i_loc, b_i_loc, i, a_i_loc, b_i_loc, retdest
    %mload_kernel_general
    SWAP1
    %mload_kernel_general
    SWAP1
    // stack: a[i], b[i], i, a_i_loc, b_i_loc, retdest
    %stack (vals: 2) -> (vals, vals)
    GT
    %jumpi(greater)
    // stack: a[i], b[i], i, a_i_loc, b_i_loc, retdest
    LT
    %jumpi(less)
    // stack: i, a_i_loc, b_i_loc, retdest
    DUP1
    %eq_const(0)
    %jumpi(equal)
    %decrement
    // stack: i-1, a_i_loc, b_i_loc, retdest
    SWAP1
    // stack: a_i_loc, i-1, b_i_loc, retdest
    %decrement
    // stack: a_i_loc_new, i-1, b_i_loc, retdest
    SWAP2
    // stack: b_i_loc, i-1, a_i_loc_new, retdest
    %decrement
    // stack: b_i_loc_new, i-1, a_i_loc_new, retdest
    %stack (b, i, a) -> (i, a, b)
    // stack: i-1, a_i_loc_new, b_i_loc_new, retdest
    %jump(ge_loop)
equal:
    // stack: i, a_i_loc, b_i_loc, retdest
    %stack (vals: 3) -> ()
    // stack: retdest
    PUSH 3
    // stack: 3, retdest
    SWAP1
    JUMP
greater:
    // stack: a[i], b[i], i, a_i_loc, b_i_loc, retdest
    %stack (vals: 5) -> ()
    // stack: retdest
    PUSH 1
    // stack: 1, retdest
    SWAP1
    JUMP
less:
    // stack: i, a_i_loc, b_i_loc, retdest
    %stack (vals: 3) -> ()
    // stack: retdest
    PUSH 0
    // stack: 0, retdest
    SWAP1
    JUMP

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

// Multiplies a bignum by a constant value.
bignum_mul_helper:
    // stack: n=length, i=start_loc, val, retdest
mul_helper_loop:
    // stack: n, i, val, retdest
    DUP2
    // stack: i, n, i, val, retdest
    %mload_kernel_general
    // stack: bignum[i], n, i, val, retdest
    DUP4
    // stack: val, bignum[i], n, i, val, retdest
    MUL
    // stack: val * bignum[i], n, i, val, retdest
    DUP3
    // stack: i, val * bignum[i], n, i, val, retdest
    %mstore_kernel_general
    // stack: n, i, val, retdest
    %decrement
    SWAP1
    %increment
    SWAP1
    // stack: n - 1, i + 1, val, retdest
    DUP1
    // stack: n - 1, n - 1, i + 1, val, retdest
    ISZERO
    %jumpi(mul_helper_end)
    %jump(mul_helper_loop)
mul_helper_end:
    // stack: n = 0, i, val, retdest
    %stack (vals: 3) -> ()
    // stack: retdest
    JUMP

// Reduces a bignum with limbs possibly greater than 128 bits to a normalized bignum with length (length + 1).
bignum_reduce_helper:
    // stack: length, start_loc, retdest
    %stack (vals: 2) -> (vals, 0)
    // stack: n=length, i=start_loc, carry=0, retdest
reduce_loop:
    // stack: n, i, retdest

    // stack: n, i, carry, retdest
    %decrement
    SWAP1
    %increment
    SWAP1
    // stack: n - 1, i + 1, carry, retdest
    DUP1
    // stack: n - 1, n - 1, i + 1, carry, retdest
    ISZERO
    %jumpi(reduce_end)
    %jump(reduce_loop)
reduce_end:
    // stack: n = 0, i, carry, retdest
    %stack (vals: 3) -> ()
    // stack: retdest
    JUMP

// Stores a * b in output_loc, leaving a and b unchanged.
global mul_bignum_bounded:
    // stack: length, a_start_loc, b_start_loc, output_loc, retdest
    
mul_loop:

mul_end: