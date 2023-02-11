// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Shifts a given bignum right by one bit (in place).
global shr_bignum:
    // stack: len, start_loc, retdest
    DUP2
    // stack: start_loc, len, start_loc, retdest
    ADD
    // stack: start_loc + len, start_loc, retdest
    %decrement
    // stack: end_loc, start_loc, retdest
    %stack (e) -> (e, 0)
    // stack: i=end_loc, carry=0, start_loc, retdest
shr_loop:
    // stack: i, carry, start_loc, retdest
    DUP1
    // stack: i, i, carry, start_loc, retdest
    %mload_kernel_general
    // stack: a[i], i, carry, start_loc, retdest
    DUP1
    // stack: a[i], a[i], i, carry, start_loc, retdest
    %shr_const(1)
    // stack: a[i] >> 1, a[i], i, carry, start_loc, retdest
    SWAP1
    // stack: a[i], a[i] >> 1, i, carry, start_loc, retdest
    %mod_const(2)
    // stack: new_carry = a[i] % 2, a[i] >> 1, i, carry, start_loc, retdest
    SWAP3
    // stack: carry, a[i] >> 1, i, new_carry, start_loc, retdest
    %shl_const(127)
    // stack: carry << 127, a[i] >> 1, i, new_carry, start_loc, retdest
    OR
    // stack: carry << 127 | a[i] >> 1, i, new_carry, start_loc, retdest
    DUP2
    // stack: i, carry << 127 | a[i] >> 1, i, new_carry, start_loc, retdest
    %mstore_kernel_general
    // stack: i, new_carry, start_loc, retdest
    DUP1
    // stack: i, i, new_carry, start_loc, retdest
    %decrement
    // stack: i-1, i, new_carry, start_loc, retdest
    SWAP1
    // stack: i, i-1, new_carry, start_loc, retdest
    DUP4
    // stack: start_loc, i, i-1, new_carry, start_loc, retdest
    EQ
    // stack: i == start_loc, i-1, new_carry, start_loc, retdest
    ISZERO
    // stack: i != start_loc, i-1, new_carry, start_loc, retdest
    %jumpi(shr_loop)
shr_end:
    // stack: i, new_carry, start_loc, retdest
    %pop3
    // stack: retdest
    JUMP
