// Arithmetic on little-endian integers represented with 128-bit limbs.
// All integers must be under a given length bound, and are padded with leading zeroes.

// Shifts a given bignum right by one bit (in place).
// Assumes that len > 0.
global shr_bignum:
    // stack: len, start_loc, retdest
    DUP1
    // stack: len, len, start_loc, retdest
    ISZERO
    %jumpi(len_zero)
    // stack: len, start_loc, retdest
    DUP2
    // stack: start_loc, len, start_loc, retdest
    ADD
    // stack: start_loc + len, start_loc, retdest
    %decrement
    // stack: end_loc, start_loc, retdest
    
    %build_current_general_address_no_offset

    // stack: base_addr, end_loc, start_loc, retdest
    %stack (addr, e) -> (e, addr, 0)
    // stack: i=end_loc, base_addr, carry=0, start_loc, retdest
shr_loop:
    // stack: i, base_addr, carry, start_loc, retdest
    DUP1
    // stack: i, i, base_addr, carry, start_loc, retdest
    DUP3 ADD // addr_i
    MLOAD_GENERAL
    // stack: a[i], i, base_addr, carry, start_loc, retdest
    DUP1
    // stack: a[i], a[i], i, base_addr, carry, start_loc, retdest
    %shr_const(1)
    // stack: a[i] >> 1, a[i], i, base_addr, carry, start_loc, retdest
    SWAP1
    // stack: a[i], a[i] >> 1, i, base_addr, carry, start_loc, retdest
    %mod_const(2)
    // stack: new_carry = a[i] % 2, a[i] >> 1, i, base_addr, carry, start_loc, retdest
    SWAP4
    // stack: carry, a[i] >> 1, i, base_addr, new_carry, start_loc, retdest
    %shl_const(127)
    // stack: carry << 127, a[i] >> 1, i, base_addr, new_carry, start_loc, retdest
    ADD
    // stack: carry << 127 | a[i] >> 1, i, base_addr, new_carry, start_loc, retdest
    DUP2
    // stack: i, carry << 127 | a[i] >> 1, i, base_addr, new_carry, start_loc, retdest
    DUP4 ADD // addr_i
    %swap_mstore
    // stack: i, base_addr, new_carry, start_loc, retdest
    PUSH 1
    DUP2
    SUB
    // stack: i-1, i, base_addr, new_carry, start_loc, retdest
    SWAP1
    // stack: i, i-1, base_addr, new_carry, start_loc, retdest
    DUP5
    // stack: start_loc, i, i-1, base_addr, new_carry, start_loc, retdest
    EQ
    // stack: i == start_loc, i-1, base_addr, new_carry, start_loc, retdest
    ISZERO
    // stack: i != start_loc, i-1, base_addr, new_carry, start_loc, retdest
    %jumpi(shr_loop)
shr_end:
    // stack: i, base_addr, new_carry, start_loc, retdest
    %pop4
    // stack: retdest
    JUMP

len_zero:
    // stack: len, start_loc, retdest
    %pop2
    // stack: retdest
    JUMP
