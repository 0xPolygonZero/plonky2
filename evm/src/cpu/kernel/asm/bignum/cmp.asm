// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Compares two bignums of the same given length. Assumes that len > 0.
// Returns 1 if a > b, 0 if a == b, and -1 (that is, 2^256 - 1) if a < b.
global cmp_bignum:
    // stack: len, a_start_loc, b_start_loc, retdest
    %build_current_general_address_no_offset
    // stack: base_addr, len, a_start_loc, b_start_loc, retdest
    DUP2
    // stack: len, base_addr, len, a_start_loc, b_start_loc, retdest
    ISZERO
    %jumpi(equal) // len and base_addr are swapped, but they will be popped anyway
    // stack: base_addr, len, a_start_loc, b_start_loc, retdest
    SWAP2
    // stack: a_start_loc, len, base_addr, b_start_loc, retdest
    PUSH 1
    DUP3
    SUB
    // stack: len-1, a_start_loc, len, base_addr, b_start_loc, retdest
    ADD
    // stack: a_end_loc, len, base_addr, b_start_loc, retdest
    SWAP3
    // stack: b_start_loc, len, base_addr, a_end_loc, retdest
    PUSH 1
    DUP3
    SUB
    // stack: len-1, b_start_loc, len, base_addr, a_end_loc, retdest
    ADD
    // stack: b_end_loc, len, base_addr, a_end_loc, retdest

    %stack (b, l, addr, a) -> (l, addr, a, b)
    // stack: len, base_addr, a_end_loc, b_end_loc, retdest
    %decrement
ge_loop:
    // stack: i, base_addr, a_i_loc, b_i_loc, retdest
    DUP4
    // stack: b_i_loc, i, base_addr, a_i_loc, b_i_loc, retdest
    DUP3 ADD // b_i_addr
    MLOAD_GENERAL
    // stack: b[i], i, base_addr, a_i_loc, b_i_loc, retdest
    DUP4
    // stack: a_i_loc, b[i], i, base_addr, a_i_loc, b_i_loc, retdest
    DUP4 ADD // a_i_addr
    MLOAD_GENERAL
    // stack: a[i], b[i], i, base_addr, a_i_loc, b_i_loc, retdest
    %stack (vals: 2) -> (vals, vals)
    GT
    %jumpi(greater)
    // stack: a[i], b[i], i, base_addr, a_i_loc, b_i_loc, retdest
    LT
    %jumpi(less)
    // stack: i, base_addr, a_i_loc, b_i_loc, retdest
    DUP1
    ISZERO
    %jumpi(equal)
    %decrement
    // stack: i-1, base_addr, a_i_loc, b_i_loc, retdest
    SWAP2
    // stack: a_i_loc, base_addr, i-1, b_i_loc, retdest
    %decrement
    // stack: a_i_loc_new, base_addr, i-1, b_i_loc, retdest
    SWAP3
    // stack: b_i_loc, base_addr, i-1, a_i_loc_new, retdest
    %decrement
    // stack: b_i_loc_new, base_addr, i-1, a_i_loc_new, retdest
    %stack (b, addr, i, a) -> (i, addr, a, b)
    // stack: i-1, base_addr, a_i_loc_new, b_i_loc_new, retdest
    %jump(ge_loop)
equal:
    // stack: i, base_addr, a_i_loc, b_i_loc, retdest
    %pop4
    // stack: retdest
    PUSH 0
    // stack: 0, retdest
    SWAP1
    JUMP
greater:
    // stack: a[i], b[i], i, base_addr, a_i_loc, b_i_loc, retdest
    %pop6
    // stack: retdest
    PUSH 1
    // stack: 1, retdest
    SWAP1
    JUMP
less:
    // stack: i, base_addr, a_i_loc, b_i_loc, retdest
    %pop4
    // stack: retdest
    PUSH 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
    // stack: -1, retdest
    SWAP1
    JUMP
