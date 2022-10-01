// Computes the hex-prefix encoding of the given nibble list and termination
// flag. Writes the result to the @KERNEL_GENERAL segment of memory, and returns
// its length on the stack.
global hex_prefix:
    // stack: num_nibbles, packed_nibbles, terminated, retdest
    // We will iterate backwards, from i = num_nibbles/2 to i=0, so that we can
    // take nibbles from the least-significant end of packed_nibbles.
    PUSH 2 DUP2 DIV // i = num_nibbles / 2
    // stack: i, num_nibbles, packed_nibbles, terminated, retdest

loop:
    // If i == 0, break to first_byte.
    DUP1 ISZERO %jumpi(first_byte)

    // stack: i, num_nibbles, packed_nibbles, terminated, retdest
    DUP3 // packed_nibbles
    %and_const(0xFF)
    // stack: byte_i, i, num_nibbles, packed_nibbles, terminated, retdest
    DUP2 // i
    %mstore_kernel_general
    // stack: i, num_nibbles, packed_nibbles, terminated, retdest
    %sub_const(1)
    SWAP2 %shr_const(8) SWAP2 // packed_nibbles >>= 8
    // stack: i, num_nibbles, packed_nibbles, terminated, retdest
    %jump(loop)

first_byte:
    // stack: 0, num_nibbles, first_nibble_or_zero, terminated, retdest
    POP
    DUP1
    // stack: num_nibbles, num_nibbles, first_nibble_or_zero, terminated, retdest
    %div_const(2)
    %add_const(1)
    // stack: result_len, num_nibbles, first_nibble_or_zero, terminated, retdest
    SWAP3
    // stack: terminated, num_nibbles, first_nibble_or_zero, result_len, retdest
    %mul_const(2)
    SWAP1
    // stack: num_nibbles, terminated * 2, first_nibble_or_zero, result_len, retdest
    %mod_const(2)
    ADD
    // stack: parity + terminated * 2, first_nibble_or_zero, result_len, retdest
    %mul_const(16)
    ADD
    // stack: 16 * (parity + terminated * 2) + first_nibble_or_zero, result_len, retdest
    PUSH 0
    %mstore_kernel_general

    // stack: result_len, retdest
    SWAP1
    JUMP
