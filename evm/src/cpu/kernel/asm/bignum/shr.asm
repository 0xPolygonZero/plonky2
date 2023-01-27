// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Shifts a given bignum right by one bit.
global shr_bignum:
    // stack: len, start_loc, retdest
    DUP2
    // stack: start_loc, len, start_loc, retdest
    ADD
    // stack: end_loc, start_loc, retdest
    %stack (e, s) -> (s, 0, e)
    // stack: i=start_loc, carry=0, end_loc, retdest
shr_loop:
    // stack: i, carry, end_loc, retdest
    DUP2
    // stack: i, i, carry, end_loc, retdest
    %mload_kernel_general
    // stack: a[i], i, carry, end_loc, retdest
    DUP1
    // stack: a[i], a[i], i, carry, end_loc, retdest
    %shr_const(1)
    // stack: a[i] >> 1, a[i], i, carry, end_loc, retdest
    SWAP1
    // stack: a[i], a[i] >> 1, i, carry, end_loc, retdest
    %mod_const(2)
    // stack: a[i] % 2, a[i] >> 1, i, carry, end_loc, retdest
    SWAP3
    // stack: carry, a[i] >> 1, i, new_carry = a[i] % 2, end_loc, retdest
    %shl_const(127)
    // stack: carry << 127, a[i] >> 1, i, new_carry, end_loc, retdest
    OR
    // stack: carry << 127 | a[i] >> 1, i, new_carry, end_loc, retdest
    DUP2
    // stack: i, carry << 127 | a[i] >> 1, i, new_carry, end_loc, retdest
    %mstore_kernel_general
    // stack: i, new_carry, end_loc, retdest
    %increment
    // stack: i+1, new_carry, end_loc, retdest
    DUP1
    // stack: i+1, i+1, new_carry, end_loc, retdest
    DUP4
    // stack: end_loc, i+1, i+1, new_carry, end_loc, retdest
    LT
    // stack: end_loc < i+1, i+1, new_carry, end_loc, retdest
    ISZERO
    // stack: i+1 <= end_loc, i+1, new_carry, end_loc, retdest
    %jumpi(shr_loop)
shr_end:
    // stack: i, new_carry, end_loc, retdest
    %stack (vals: 3) -> ()
    // stack: retdest
    JUMP
