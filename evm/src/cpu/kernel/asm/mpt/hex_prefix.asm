global debug_compare_results:
    // stac: new_rlp_pos, new_another_rlp_pos, rlp_pos, another_rlp_pos, ret_dest
    %stack
        (new_rlp_pos, new_another_rlp_pos, rlp_pos, another_rlp_pos, ret_dest) ->
        (new_rlp_pos, new_another_rlp_pos, rlp_pos, another_rlp_pos, ret_dest, new_rlp_pos)
    DUP3 SWAP1 SUB
    // stack: delta, new_another_rlp_pos, rlp_pos, another_rlp_pos, ret_dest, new_rlp_pos
    SWAP1 DUP4 SWAP1 SUB DUP2
    // stack: delta, another_delta, delta, rlp_pos, another_rlp_pos, ret_dest, new_rlp_pos
    %assert_eq
global debug_compare_loop:
    // stack: delta, rlp_pos, another_rlp_pos, ret_dest, new_rlp_pos
    DUP1 ISZERO %jumpi(debug_compare_end)
    %stack 
        (delta, rlp_pos, another_rlp_pos) ->
        (rlp_pos, another_rlp_pos, rlp_pos, another_rlp_pos, delta)
global debug_before_read_original:
    %mload_kernel(@SEGMENT_RLP_RAW)
    SWAP1
global debug_before_read_new:
    %mload_kernel(@SEGMENT_RLP_RAW)
global debug_before_compare_read:
    %assert_eq
    %increment SWAP1 %increment SWAP2 %decrement
global debug_before_jump:
    %jump(debug_compare_loop)
global debug_compare_end:
    %stack 
        (delta, rlp_pos, another_rlp_pos, ret_dest, new_rlp_pos) ->
        (ret_dest, new_rlp_pos)
    JUMP



// Computes the RLP encoding of the hex-prefix encoding of the given nibble list
// and termination flag. Writes the result to @SEGMENT_RLP_RAW starting at the
// given position, and returns the updated position, i.e. a pointer to the next
// unused offset.
//
// Pre stack: rlp_start_pos, num_nibbles, packed_nibbles, terminated, retdest
// Post stack: rlp_end_pos

global hex_prefix_rlp_new:
    // TODO: Remove this
    %alloc_rlp_block
    %stack
        (another_rlp_pos, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest) ->
        (another_rlp_pos, num_nibbles, packed_nibbles, terminated, debug_hex_prefix_rlp, another_rlp_pos, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest)
    %jump(hex_prefix_rlp_new)
global debug_hex_prefix_rlp:
    // stack: new_another_rlp_pos, another_rlp_pos, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest
    %stack
        (new_another_rlp_pos, another_rlp_pos, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest) ->
        (rlp_pos, num_nibbles, packed_nibbles, terminated, debug_compare_results, new_another_rlp_pos, rlp_pos, another_rlp_pos, retdest)

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

global rlp_header_medium:
    // stack: hp_len, i, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest
    DUP1 %add_const(0x80) // value = 0x80 + hp_len
    DUP4 // offset = rlp_pos
global debug_before_write_rlp_med_orginal:
    %mstore_rlp

    // rlp_pos += 1
    SWAP2 %increment SWAP2

    %jump(start_loop)

global rlp_header_large:
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

// Pre stack: rlp_start_pos, num_nibbles, packed_nibbles, terminated, retdest
// Post stack: rlp_end_pos
global hex_prefix_rlp:
    DUP2 %assert_lt_const(65) // TODO: The number of nibbles can't be more than 64?
    
    PUSH 2 DUP3 DIV 
    // Compute the length of the hex-prefix string, in bytes:
    // hp_len = num_nibbles / 2 + 1 = i + 1
    %increment
    // stack: hp_len, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest

    // Write the RLP header.
    DUP1 %gt_const(55) %jumpi(rlp_header_large_new)
    DUP1 %gt_const(1) %jumpi(rlp_header_medium_new)

    // The hex-prefix is a single byte. It must be <= 127, since its first
    // nibble only has two bits. So this is the "small" RLP string case, where
    // the byte is its own RLP encoding.
    // stack: hp_len, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest
    POP
global first_byte_new:
    // stack: rlp_pos, num_nibbles, packed_nibbles, terminated, retdest
    // get the first nibble, if num_nibbles is odd, or zero otherwise
    SWAP2
    // stack: packed_nibbles, num_nibbbles, rlp_pos, terminated, retdest
    DUP2 DUP1
    %mod_const(2)
    // stack: parity, num_nibbles, packed_nibbles, num_nibbles, rlp_pos, terminated, retdest
    SWAP1 SUB
    %mul_const(4)
    SHR
    // stack: first_nibble_or_zero, num_nibbles, rlp_pos, terminated, retdest
    SWAP2
    // stack: rlp_pos, num_nibbles, first_nibble_or_zero, terminated, retdest
    SWAP3
    // stack: terminated, num_nibbles, first_nibble_or_zero, rlp_pos, retdest
    %mul_const(2)
    // stack: terminated * 2, num_nibbles, first_nibble_or_zero, rlp_pos, retdest
    SWAP1
    // stack: num_nibbles, terminated * 2, first_nibble_or_zero, rlp_pos, retdest
    %mod_const(2) // parity
    ADD
    // stack: parity + terminated * 2, first_nibble_or_zero, rlp_pos, retdest
    %mul_const(16)
    ADD
    // stack: first_byte, rlp_pos, retdest
    DUP2
    %mstore_rlp
    %increment
    // stack: rlp_pos, retdest
    SWAP1
    JUMP
    
global remaining_bytes:
    // stack: rlp_pos, num_nibbles, packed_nibbles, retdest
    SWAP2
    PUSH 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF
    // stack: 1^256, packed_nibbles, num_nibbles, rlp_pos, ret_dest
    SWAP1 SWAP2 DUP1
    %mod_const(2)
    // stack: parity, num_nibbles, 1^256, packed_nibbles, rlp_pos, ret_dest
    SWAP1 SUB DUP1
    // stack: num_nibbles - parity, num_nibbles - parity, 1^256, packed_nibbles, rlp_pos, ret_dest
    %div_const(2)
    // stack: remaining_bytes, num_nibbles - parity, 1^256, packed_nibbles, rlp_pos, ret_dest
    SWAP2 SWAP1
    // stack: num_nibbles - parity, 1^256, remaining_bytes, packed_nibbles, rlp_pos, ret_dest
    %mul_const(4)
    // stack: 4*(num_nibbles - parity), 1^256, remaining_bytes, packed_nibbles, rlp_pos, ret_dest
    PUSH 256 SUB
    // stack: 256 - 4*(num_nibbles - parity), 1^256, remaining_bytes, packed_nibbles, rlp_pos, ret_dest
    SHR
    // stack mask, remaining_bytes, packed_nibbles, rlp_pos, ret_dest
    SWAP1 SWAP2
    AND
    %stack
        (remaining_nibbles, remaining_bytes, rlp_pos) ->
        (rlp_pos, remaining_nibbles, remaining_bytes)
    %mstore_unpacking_rlp
global debug_affter_mstore_unpacking:
    SWAP1
    JUMP


global rlp_header_medium_new:
    // stack: hp_len, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest
    %add_const(0x80) // value = 0x80 + hp_len
    DUP2 // offset = rlp_pos
global debug_before_write_rlp_med_new:
    %mstore_rlp
    // stack: rlp_pos, num_nibbles, packed_nibbles, terminated, retdest
    // rlp_pos += 1
    %increment

    %stack
        (rlp_pos, num_nibbles, packed_nibbles, terminated, retdest) ->
        (rlp_pos, num_nibbles, packed_nibbles, terminated, remaining_bytes, num_nibbles, packed_nibbles, retdest)

    %jump(first_byte_new)

global rlp_header_large_new:
    // stack: hp_len, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest
    // In practice hex-prefix length will never exceed 256, so the length of the
    // length will always be 1 byte in this case.

    PUSH 0xb8 // value = 0xb7 + len_of_len = 0xb8
    DUP3 // offset = rlp_pos
    %mstore_rlp

    // stack: hp_len, rlp_pos, num_nibbles, packed_nibbles, terminated, retdest
    DUP2 %increment // offset = rlp_pos + 1
    %mstore_rlp

    // stack rlp_pos, num_nibbles, packed_nibbles, terminated, retdest
    // rlp_pos += 2
    %add_const(2)

    %stack
        (rlp_pos, num_nibbles, packed_nibbles, terminated, retdest) ->
        (rlp_pos, num_nibbles, packed_nibbles, terminated, remaining_bytes, num_nibbles, packed_nibbles, retdest)

    %jump(first_byte_new)

