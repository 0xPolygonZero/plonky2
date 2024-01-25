// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

global isone_bignum:
    // stack: len, start_loc, retdest
    DUP1
    // stack: len, len, start_loc, retdest
    ISZERO
    %jumpi(eqzero)
    // stack: len, start_loc, retdest
    DUP2
    // stack: start_loc, len, start_loc, retdest
    %mload_current_general
    // stack: start_val, len, start_loc, retdest
    %eq_const(1)
    %jumpi(starts_with_one)
    // Does not start with one, so not equal to one.
    // stack: len, start_loc, retdest
    %stack (vals: 2, retdest) -> (retdest, 0)
    JUMP
eqzero:
    // Is zero, so not equal to one.
    // stack: cur_loc, end_loc, retdest
    %stack (vals: 2, retdest) -> (retdest, 0)
    // stack: retdest, 0
    JUMP
starts_with_one:
    // Starts with one, so check that the remaining limbs are zero.
    // stack: len, start_loc, retdest
    %decrement
    SWAP1
    %increment
    SWAP1
    // stack: len-1, start_loc+1, retdest
    %jump(iszero_bignum)
