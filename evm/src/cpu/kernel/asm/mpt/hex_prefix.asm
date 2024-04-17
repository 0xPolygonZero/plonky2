// Computes the RLP encoding of the hex-prefix encoding of the given nibble list
// and termination flag. Writes the result to @SEGMENT_RLP_RAW starting at the
// given position, and returns the updated position, i.e. a pointer to the next
// unused offset.
//
// Pre stack: rlp_start_addr, num_nibbles, packed_nibbles, terminated, retdest
// Post stack: rlp_end_addr
global hex_prefix_rlp:
    DUP2 %assert_lt_const(65)
    
    PUSH 2 DUP3 DIV 
    // Compute the length of the hex-prefix string, in bytes:
    // hp_len = num_nibbles / 2 + 1 = i + 1
    %increment
    // stack: hp_len, rlp_addr, num_nibbles, packed_nibbles, terminated, retdest

    // Write the RLP header.
    DUP1 %gt_const(55) %jumpi(rlp_header_large)
    DUP1 %gt_const(1) %jumpi(rlp_header_medium)

    // The hex-prefix is a single byte. It must be <= 127, since its first
    // nibble only has two bits. So this is the "small" RLP string case, where
    // the byte is its own RLP encoding.
    // stack: hp_len, rlp_addr, num_nibbles, packed_nibbles, terminated, retdest
    POP
first_byte:
    // stack: rlp_addr, num_nibbles, packed_nibbles, terminated, retdest
    // get the first nibble, if num_nibbles is odd, or zero otherwise
    SWAP2
    // stack: packed_nibbles, num_nibbles, rlp_addr, terminated, retdest
    DUP2 DUP1
    %mod_const(2)
    // stack: parity, num_nibbles, packed_nibbles, num_nibbles, rlp_addr, terminated, retdest
    SWAP1 SUB
    %mul_const(4)
    SHR
    // stack: first_nibble_or_zero, num_nibbles, rlp_addr, terminated, retdest
    SWAP2
    // stack: rlp_addr, num_nibbles, first_nibble_or_zero, terminated, retdest
    SWAP3
    // stack: terminated, num_nibbles, first_nibble_or_zero, rlp_addr, retdest
    %mul_const(2)
    // stack: terminated * 2, num_nibbles, first_nibble_or_zero, rlp_addr, retdest
    SWAP1
    // stack: num_nibbles, terminated * 2, first_nibble_or_zero, rlp_addr, retdest
    %mod_const(2) // parity
    ADD
    // stack: parity + terminated * 2, first_nibble_or_zero, rlp_addr, retdest
    %mul_const(16)
    ADD
    // stack: first_byte, rlp_addr, retdest
    DUP2
    %swap_mstore
    %increment
    // stack: rlp_addr', retdest
    SWAP1
    JUMP
    
remaining_bytes:
    // stack: rlp_addr, num_nibbles, packed_nibbles, retdest
    SWAP2
    PUSH @U256_MAX
    // stack: U256_MAX, packed_nibbles, num_nibbles, rlp_addr, ret_dest
    SWAP1 SWAP2 DUP1
    %mod_const(2)
    // stack: parity, num_nibbles, U256_MAX, packed_nibbles, rlp_addr, ret_dest
    SWAP1 SUB DUP1
    // stack: num_nibbles - parity, num_nibbles - parity, U256_MAX, packed_nibbles, rlp_addr, ret_dest
    %div_const(2)
    // stack: rem_bytes, num_nibbles - parity, U256_MAX, packed_nibbles, rlp_addr, ret_dest
    SWAP2 SWAP1
    // stack: num_nibbles - parity, U256_MAX, rem_bytes, packed_nibbles, rlp_addr, ret_dest
    %mul_const(4)
    // stack: 4*(num_nibbles - parity), U256_MAX, rem_bytes, packed_nibbles, rlp_addr, ret_dest
    PUSH 256 SUB
    // stack: 256 - 4*(num_nibbles - parity), U256_MAX, rem_bytes, packed_nibbles, rlp_addr, ret_dest
    SHR
    // stack: mask, rem_bytes, packed_nibbles, rlp_addr, ret_dest
    SWAP1 SWAP2
    AND
    %stack(remaining_nibbles, rem_bytes, rlp_addr) -> (rlp_addr, remaining_nibbles, rem_bytes)
    %mstore_unpacking
    SWAP1
    JUMP


rlp_header_medium:
    // stack: hp_len, rlp_addr, num_nibbles, packed_nibbles, terminated, retdest
    %add_const(0x80) // value = 0x80 + hp_len
    DUP2
    %swap_mstore
    // stack: rlp_addr, num_nibbles, packed_nibbles, terminated, retdest
    // rlp_addr += 1
    %increment

    // stack: rlp_addr, num_nibbles, packed_nibbles, terminated, retdest
    SWAP3 DUP3 DUP3
    // stack: num_nibbles, packed_nibbles, terminated, num_nibbles, packed_nibbles, rlp_addr, retdest
    PUSH remaining_bytes
    // stack: remaining_bytes, num_nibbles, packed_nibbles, terminated, num_nibbles, packed_nibbles, rlp_addr, retdest
    SWAP4 SWAP5 SWAP6
    // stack: rlp_addr, num_nibbles, packed_nibbles, terminated, remaining_bytes, num_nibbles, packed_nibbles, retdest

    %jump(first_byte)

rlp_header_large:
    // stack: hp_len, rlp_addr, num_nibbles, packed_nibbles, terminated, retdest
    // In practice hex-prefix length will never exceed 256, so the length of the
    // length will always be 1 byte in this case.

    DUP2 // rlp_addr
    PUSH 0xb8 // value = 0xb7 + len_of_len = 0xb8
    MSTORE_GENERAL
    // stack: rlp_addr, value, hp_len, i, rlp_addr, num_nibbles, packed_nibbles, terminated, retdest

    // stack: hp_len, rlp_addr, num_nibbles, packed_nibbles, terminated, retdest
    DUP2 %increment
    %swap_mstore

    // stack: rlp_addr, num_nibbles, packed_nibbles, terminated, retdest
    // rlp_addr += 2
    %add_const(2)

    // stack: rlp_addr, num_nibbles, packed_nibbles, terminated, retdest
    SWAP3 DUP3 DUP3
    // stack: num_nibbles, packed_nibbles, terminated, num_nibbles, packed_nibbles, rlp_addr, retdest
    PUSH remaining_bytes
    // stack: remaining_bytes, num_nibbles, packed_nibbles, terminated, num_nibbles, packed_nibbles, rlp_addr, retdest
    SWAP4 SWAP5 SWAP6
    // stack: rlp_addr, num_nibbles, packed_nibbles, terminated, remaining_bytes, num_nibbles, packed_nibbles, retdest

    %jump(first_byte)
