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
    STOP
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


//// Replaces a with a + b, leaving b unchanged.
//global add_bignum_bounded:
//    // stack: a_len, b_len, a_start_loc, b_start_loc, retdest
//    %stack (al, bl, a, b) -> (0, 0, a, b, bl)
//    // stack: carry=0, i=0, a_start_loc, b_start_loc, n=b_len, retdest
//add_loop:
//    // stack: carry, i, a_i_loc, b_i_loc, n, retdest
//    DUP4
//    %mload_kernel_general
//    // stack: b[i], carry, i, a_i_loc, b_i_loc, n, retdest
//    DUP4
//    %mload_kernel_general
//    // stack: a[i], b[i], carry, i, a_i_loc, b_i_loc, n, retdest
//    ADD
//    ADD
//    // stack: a[i] + b[i] + carry, i, a_i_loc, b_i_loc, n, retdest
//    %stack (val) -> (val, @LIMB_BASE, @LIMB_BASE, val)
//    // stack: a[i] + b[i] + carry, 2^128, 2^128, a[i] + b[i] + carry, i, a_i_loc, b_i_loc, n, retdest
//    DIV
//    // stack: (a[i] + b[i] + carry) // 2^128, 2^128, a[i] + b[i] + carry, i, a_i_loc, b_i_loc, n, retdest
//    SWAP2
//    // stack: a[i] + b[i] + carry, 2^128, (a[i] + b[i] + carry) // 2^128, i, a_i_loc, b_i_loc, n, retdest
//    MOD
//    // stack: c[i] = (a[i] + b[i] + carry) % 2^128, carry_new = (a[i] + b[i] + carry) // 2^128, i, a_i_loc, b_i_loc, n, retdest
//    DUP4
//    // stack: a_i_loc, c[i], carry_new, i, a_i_loc, b_i_loc, n, retdest
//    %mstore_kernel_general
//    // stack: carry_new, i, a_i_loc, b_i_loc, n, retdest
//    %stack (c, i, a, b) -> (a, b, c, i)
//    // stack: a_i_loc, b_i_loc, carry_new, i, n, retdest
//    %increment
//    SWAP1
//    %increment
//    SWAP1
//    %stack (a, b, c, i) -> (c, i, a, b)
//    // stack: carry_new, i, a_i_loc + 1, b_i_loc + 1, n, retdest
//    SWAP1
//    %increment
//    SWAP1
//    // stack: carry_new, i + 1, a_i_loc + 1, b_i_loc + 1, n, retdest
//    DUP5
//    DUP3
//    // stack: i + 1, n, carry_new, i + 1, a_i_loc + 1, b_i_loc + 1, n, retdest
//    EQ
//    ISZERO
//    %jumpi(add_loop)
//add_end:
//    // stack: carry_new, i + 1, a_i_loc + 1, b_i_loc + 1, n, retdest
//    %stack (c, i, a, b, n) -> (c, a)
//    // stack: carry_new, a_i_loc + 1, retdest
//    // If carry = 0, no need to decrement.
//    ISZERO
//    %jumpi(increment_end)
//increment_loop:
//    // stack: cur_loc, retdest
//    DUP1
//    %mload_kernel_general
//    // stack: val, cur_loc, retdest
//    %increment
//    // stack: val+1, cur_loc, retdest
//    DUP2
//    // stack: cur_loc, val+1, cur_loc, val+1, retdest
//    %mstore_kernel_general
//    // stack: cur_loc, val+1, retdest
//    %increment
//    // stack: cur_loc + 1, val+1, retdest
//    SWAP1
//    // stack: val+1, cur_loc + 1, retdest
//    %eq_const(@LIMB_BASE)
//    ISZERO
//    %jumpi(increment_end)
//    // stack: cur_loc + 1, retdest
//    PUSH 0
//    DUP2
//    // stack: cur_loc + 1, 0, cur_loc + 1, retdest
//    %mstore_kernel_general
//    %jump(increment_loop)
//increment_end:
//    // cur_loc, retdest
//    POP
//    // retdest
//    JUMP
//
//%macro subtract_limb
//    // stack: a_i, b_i, borrow
//    DUP3
//    DUP2
//    SUB
//    // stack: a_i - borrow, a_i, b_i, borrow
//    DUP3
//    // stack: b_i, a_i - borrow, a_i, b_i, borrow
//    GT
//    // stack: borrow_new, a_i, b_i, borrow
//    DUP1
//    PUSH @LIMB_BASE
//    MUL
//    // stack: to_add, borrow_new, a_i, b_i, borrow
//    %stack (t, bn, other: 3) -> (t, other, bn)
//    // stack: to_add, a_i, b_i, borrow, borrow_new
//    ADD
//    SUB
//    SUB
//    // stack: c_i, borrow_new
//%endmacro
//
//// Replaces a with a - b, leaving b unchanged.
//// Assumes a >= b.
//global sub_bignum_bounded:
//    // stack: a_len, b_len, a_start_loc, b_start_loc, retdest
//    %stack (al, bl, a, b) -> (0, 0, a, b, bl)
//    // stack: borrow=0, i=0, a_start_loc, b_start_loc, n=b_len, retdest
//sub_loop:
//    // stack: borrow, i, a_i_loc, b_i_loc, n, retdest
//    DUP4
//    %mload_kernel_general
//    // stack: b[i], borrow, i, a_i_loc, b_i_loc, n, retdest
//    DUP4
//    %mload_kernel_general
//    // stack: a[i], b[i], borrow, i, a_i_loc, b_i_loc, n, retdest
//    %subtract_limb
//    // stack: c[i], borrow_new, i, a_i_loc, b_i_loc, n, retdest
//    DUP4
//    // stack: a_i_loc, c[i], borrow_new, i, a_i_loc, b_i_loc, n, retdest
//    %mstore_kernel_general
//    // stack: borrow_new, i, a_i_loc, b_i_loc, n, retdest
//    %stack (bn, i, a, b) -> (a, b, bn, i)
//    // stack: a_i_loc, b_i_loc, borrow_new, i, n, retdest
//    %increment
//    SWAP1
//    %increment
//    SWAP1
//    %stack (a, b, bn, i) -> (bn, i, a, b)
//    // stack: borrow_new, i, a_i_loc + 1, b_i_loc + 1, n, retdest
//    SWAP1
//    %increment
//    SWAP1
//    // stack: borrow_new, i + 1, a_i_loc + 1, b_i_loc + 1, n, retdest
//    DUP5
//    DUP3
//    // stack: i + 1, n, borrow_new, i + 1, a_i_loc + 1, b_i_loc + 1, n, retdest
//    EQ
//    ISZERO
//    %jumpi(sub_loop)
//sub_end:
//    // stack: borrow_new, i + 1, a_i_loc + 1, b_i_loc + 1, n, retdest
//    %stack (bn, i, a, b, n) -> (bn, a)
//    // stack: borrow_new, a_i_loc + 1, retdest
//    // If borrow = 0, no need to decrement.
//    ISZERO
//    %jumpi(decrement_end)
//decrement_loop:
//    // If borrow = 1, we need to subtract 1 from the prior limb of a.
//    // stack: cur_loc, retdest
//    DUP1
//    %mload_kernel_general
//    // stack: val, cur_loc, retdest
//    %decrement
//    // stack: val-1, cur_loc, retdest
//    %stack (v, l) -> (l, v, l, v)
//    DUP2
//    // stack: cur_loc, val-1, cur_loc, val-1, retdest
//    %mstore_kernel_general
//    // stack: cur_loc, val-1, retdest
//    %increment
//    // stack: cur_loc + 1, val-1, retdest
//    SWAP1
//    // stack: val-1, cur_loc + 1, retdest
//    %increment
//    %eq_const(0)
//    NOT
//    %jumpi(decrement_end)
//    // stack: cur_loc + 1, retdest
//    PUSH 255
//    DUP2
//    // stack: cur_loc + 1, 0, cur_loc + 1, retdest
//    %mstore_kernel_general
//    %jump(decrement_loop)
//decrement_end:
//    // cur_loc, retdest
//    POP
//    // retdest
//    JUMP
