// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Return a >= b.
global ge_bignum:
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
