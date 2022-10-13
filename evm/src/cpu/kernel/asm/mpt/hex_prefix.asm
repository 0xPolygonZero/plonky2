// Computes the RLP encoding of the hex-prefix encoding of the given nibble list
// and termination flag. Writes the result to @SEGMENT_RLP_RAW starting at the
// given position, and returns the updated position, i.e. a pointer to the next
// unused offset.
//
// Pre stack: rlp_start_pos, num_nibbles, packed_nibbles, terminated, retdest
// Post stack: rlp_end_pos

global hex_prefix_rlp:
    // stack: rlp_pos, num_nibbles, packed_nibbles, terminated, retdest
    // We will iterate backwards, from i = num_nibbles / 2 to i = 0, so that we
    // can take nibbles from the least-significant end of packed_nibbles.
    PUSH 2 DUP3 DIV // i = num_nibbles / 2
    // stack: i, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest

    // Compute the length of the hex-prefix string, in bytes:
    // hp_len = num_nibbles / 2 + 1 = i + 1
    DUP1 %increment
    // stack: hp_len, i, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest

    // Write the RLP header.
    DUP1 %gt_const(55) %jumpi(rlp_header_large)
    DUP1 %gt_const(1) %jumpi(rlp_header_medium)

    // The hex-prefix is a single byte. It must be <= 127, since its first
    // nibble only has two bits. So this is the "small" RLP string case, where
    // the byte is its own RLP encoding.
    // stack: hp_len, i, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest
    %jump(start_loop)

rlp_header_medium:
    // stack: hp_len, i, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest
    DUP1 %add_const(0x80) // value = 0x80 + hp_len
    DUP4 // offset = rlp_pos
    %mstore_rlp

    // rlp_pos += 1
    SWAP2 %increment SWAP2

    %jump(start_loop)

rlp_header_large:
    // stack: hp_len, i, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest
    // In practice hex-prefix length will never exceed 256, so the length of the
    // length will always be 1 byte in this case.

    PUSH 0xb8 // value = 0xb7 + len_of_len = 0xb8
    DUP4 // offset = rlp_pos
    %mstore_rlp

    DUP1 // value = hp_len
    DUP4 %increment // offset = rlp_pos + 1
    %mstore_rlp

    // rlp_pos += 2
    SWAP2 %add_const(2) SWAP2

start_loop:
    // stack: hp_len, i, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest
    SWAP1

loop:
    // stack: i, hp_len, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest
    // If i == 0, break to first_byte.
    DUP1 ISZERO %jumpi(first_byte)

    // stack: i, hp_len, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest
    DUP5 // packed_nibbles
    %and_const(0xFF)
    // stack: byte_i, i, hp_len, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest
    DUP4 // rlp_pos
    DUP3 // i
    ADD // We'll write to offset rlp_pos + i
    %mstore_rlp

    // stack: i, hp_len, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest
    %decrement
    SWAP4 %shr_const(8) SWAP4 // packed_nibbles >>= 8
    %jump(loop)

first_byte:
    // stack: 0, hp_len, rlp_pos, num_nibbles, first_nibble_or_zero, terminated, retdest
    POP
    // stack: hp_len, rlp_pos, num_nibbles, first_nibble_or_zero, terminated, retdest
    DUP2 ADD
    // stack: rlp_end_pos, rlp_pos, num_nibbles, first_nibble_or_zero, terminated, retdest
    SWAP4
    // stack: terminated, rlp_pos, num_nibbles, first_nibble_or_zero, rlp_end_pos, retdest
    %mul_const(2)
    // stack: terminated * 2, rlp_pos, num_nibbles, first_nibble_or_zero, rlp_end_pos, retdest
    %stack (terminated_x2, rlp_pos, num_nibbles, first_nibble_or_zero)
        -> (num_nibbles, terminated_x2, first_nibble_or_zero, rlp_pos)
    // stack: num_nibbles, terminated * 2, first_nibble_or_zero, rlp_pos, rlp_end_pos, retdest
    %mod_const(2) // parity
    ADD
    // stack: parity + terminated * 2, first_nibble_or_zero, rlp_pos, rlp_end_pos, retdest
    %mul_const(16)
    ADD
    // stack: first_byte, rlp_pos, rlp_end_pos, retdest
    SWAP1
    %mstore_rlp
    // stack: rlp_end_pos, retdest
    SWAP1
    JUMP
