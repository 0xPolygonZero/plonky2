// Encodes an arbitrary string, given a pointer and length.
// Pre stack: rlp_addr, ADDR, len, retdest
// Post stack: rlp_addr'
global encode_rlp_string:
    // stack: rlp_addr, ADDR, len, retdest
    DUP3 %eq_const(1)
    // stack: len == 1, rlp_addr, ADDR, len, retdest
    DUP3
    MLOAD_GENERAL
    // stack: first_byte, len == 1, rlp_addr, ADDR, len, retdest
    %lt_const(128)
    MUL // cheaper than AND
    // stack: single_small_byte, rlp_addr, ADDR, len, retdest
    %jumpi(encode_rlp_string_small_single_byte)

    // stack: rlp_addr, ADDR, len, retdest
    DUP3 %gt_const(55)
    // stack: len > 55, rlp_addr, ADDR, len, retdest
    %jumpi(encode_rlp_string_large)

global encode_rlp_string_small:
    // stack: rlp_addr, ADDR, len, retdest
    DUP1
    DUP4 // len
    %add_const(0x80)
    // stack: first_byte, rlp_addr, rlp_addr, ADDR, len, retdest
    MSTORE_GENERAL
    // stack: rlp_addr, ADDR, len, retdest
    %increment
    // stack: rlp_addr', ADDR, len, retdest
    DUP3 DUP2 ADD // rlp_addr'' = rlp_addr' + len
    // stack: rlp_addr'', rlp_addr', ADDR, len, retdest
    %stack (rlp_addr2, rlp_addr1, ADDR, len, retdest)
        -> (rlp_addr1, ADDR, len, retdest, rlp_addr2)
    %jump(memcpy_bytes)

global encode_rlp_string_small_single_byte:
    // stack: rlp_addr, ADDR, len, retdest
    %stack (rlp_addr, ADDR, len) -> (ADDR, rlp_addr)
    MLOAD_GENERAL
    // stack: byte, rlp_addr, retdest
    DUP2 SWAP1
    MSTORE_GENERAL
    // stack: rlp_addr, retdest
    %increment
    SWAP1
    // stack: retdest, rlp_addr'
    JUMP

global encode_rlp_string_large:
    // stack: rlp_addr, ADDR, len, retdest
    DUP3 %num_bytes
    // stack: len_of_len, rlp_addr, ADDR, len, retdest
    SWAP1
    DUP1
    // stack: rlp_addr, rlp_addr, len_of_len, ADDR, len, retdest
    DUP3 // len_of_len
    %add_const(0xb7)
    // stack: first_byte, rlp_addr, rlp_addr, len_of_len, ADDR, len, retdest
    MSTORE_GENERAL
    // stack: rlp_addr, len_of_len, ADDR, len, retdest
    %increment
    // stack: rlp_addr', len_of_len, ADDR, len, retdest
    %stack (rlp_addr, len_of_len, ADDR, len)
        -> (rlp_addr, len, len_of_len, encode_rlp_string_large_after_writing_len, ADDR, len)
    %jump(mstore_unpacking)
global encode_rlp_string_large_after_writing_len:
    // stack: rlp_addr'', ADDR, len, retdest
    DUP3 DUP2 ADD // rlp_addr''' = rlp_addr'' + len
    // stack: rlp_addr''', rlp_addr'', ADDR, len, retdest
    %stack (rlp_addr3, rlp_addr2, ADDR, len, retdest)
        -> (rlp_addr2, ADDR, len, retdest, rlp_addr3)
    %jump(memcpy_bytes)

%macro encode_rlp_string
    %stack (rlp_addr, ADDR, len) -> (rlp_addr, ADDR, len, %%after)
    %jump(encode_rlp_string)
%%after:
%endmacro
