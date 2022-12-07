// Encodes an arbitrary string, given a pointer and length.
// Pre stack: pos, ADDR: 3, len, retdest
// Post stack: pos'
global encode_rlp_string:
    // stack: pos, ADDR: 3, len, retdest
    DUP5 %eq_const(1)
    // stack: len == 1, pos, ADDR: 3, len, retdest
    DUP5 DUP5 DUP5 // ADDR: 3
    MLOAD_GENERAL
    // stack: first_byte, len == 1, pos, ADDR: 3, len, retdest
    %lt_const(128)
    MUL // cheaper than AND
    // stack: single_small_byte, pos, ADDR: 3, len, retdest
    %jumpi(encode_rlp_string_small_single_byte)

    // stack: pos, ADDR: 3, len, retdest
    DUP5 %gt_const(55)
    // stack: len > 55, pos, ADDR: 3, len, retdest
    %jumpi(encode_rlp_string_large)

global encode_rlp_string_small:
    // stack: pos, ADDR: 3, len, retdest
    DUP5 // len
    %add_const(0x80)
    // stack: first_byte, pos, ADDR: 3, len, retdest
    DUP2
    // stack: pos, first_byte, pos, ADDR: 3, len, retdest
    %mstore_rlp
    // stack: pos, ADDR: 3, len, retdest
    %increment
    // stack: pos', ADDR: 3, len, retdest
    DUP5 DUP2 ADD // pos'' = pos' + len
    // stack: pos'', pos', ADDR: 3, len, retdest
    %stack (pos2, pos1, ADDR: 3, len, retdest)
        -> (0, @SEGMENT_RLP_RAW, pos1, ADDR, len, retdest, pos2)
    %jump(memcpy)

global encode_rlp_string_small_single_byte:
    // stack: pos, ADDR: 3, len, retdest
    %stack (pos, ADDR: 3, len) -> (ADDR, pos)
    MLOAD_GENERAL
    // stack: byte, pos, retdest
    DUP2
    %mstore_rlp
    // stack: pos, retdest
    %increment
    JUMP

global encode_rlp_string_large:
    // stack: pos, ADDR: 3, len, retdest
    DUP5 %num_bytes
    // stack: len_of_len, pos, ADDR: 3, len, retdest
    SWAP1
    DUP2 // len_of_len
    %add_const(0xb7)
    // stack: first_byte, pos, len_of_len, ADDR: 3, len, retdest
    DUP2
    // stack: pos, first_byte, pos, len_of_len, ADDR: 3, len, retdest
    %mstore_rlp
    // stack: pos, len_of_len, ADDR: 3, len, retdest
    %increment
    // stack: pos', len_of_len, ADDR: 3, len, retdest
    %stack (pos, len_of_len, ADDR: 3, len)
        -> (pos, len, len_of_len, encode_rlp_string_large_after_writing_len, ADDR, len)
    %jump(mstore_unpacking_rlp)
global encode_rlp_string_large_after_writing_len:
    // stack: pos'', ADDR: 3, len, retdest
    DUP5 DUP2 ADD // pos''' = pos'' + len
    // stack: pos''', pos'', ADDR: 3, len, retdest
    %stack (pos3, pos2, ADDR: 3, len, retdest)
        -> (0, @SEGMENT_RLP_RAW, pos2, ADDR, len, retdest, pos3)
    %jump(memcpy)
