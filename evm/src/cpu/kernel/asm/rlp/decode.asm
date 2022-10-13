// Note: currently, these methods do not check that RLP input is in canonical
// form; for example a single byte could be encoded with the length-of-length
// form. Technically an EVM must perform these checks, but we aren't really
// concerned with it in our setting. An attacker who corrupted consensus could
// prove a non-canonical state, but this would just temporarily stall the bridge
// until a fix was deployed. We are more concerned with preventing any theft of
// assets.

// Parse the length of a bytestring from RLP memory. The next len bytes after
// pos' will contain the string.
//
// Pre stack: pos, retdest
// Post stack: pos', len
global decode_rlp_string_len:
    // stack: pos, retdest
    DUP1
    %mload_current(@SEGMENT_RLP_RAW)
    // stack: first_byte, pos, retdest
    DUP1
    %gt_const(0xb7)
    // stack: first_byte >= 0xb8, first_byte, pos, retdest
    %jumpi(decode_rlp_string_len_large)
    // stack: first_byte, pos, retdest
    DUP1
    %gt_const(0x7f)
    // stack: first_byte >= 0x80, first_byte, pos, retdest
    %jumpi(decode_rlp_string_len_medium)

    // String is a single byte in the range [0x00, 0x7f].
    %stack (first_byte, pos, retdest) -> (retdest, pos, 1)
    JUMP

decode_rlp_string_len_medium:
    // String is 0-55 bytes long. First byte contains the len.
    // stack: first_byte, pos, retdest
    %sub_const(0x80)
    // stack: len, pos, retdest
    SWAP1
    %increment
    // stack: pos', len, retdest
    %stack (pos, len, retdest) -> (retdest, pos, len)
    JUMP

decode_rlp_string_len_large:
    // String is >55 bytes long. First byte contains the len of the len.
    // stack: first_byte, pos, retdest
    %sub_const(0xb7)
    // stack: len_of_len, pos, retdest
    SWAP1
    %increment
    // stack: pos', len_of_len, retdest
    %jump(decode_int_given_len)

// Convenience macro to call decode_rlp_string_len and return where we left off.
%macro decode_rlp_string_len
    %stack (pos) -> (pos, %%after)
    %jump(decode_rlp_string_len)
%%after:
%endmacro

// Parse a scalar from RLP memory.
// Pre stack: pos, retdest
// Post stack: pos', scalar
//
// Scalars are variable-length, but this method assumes a max length of 32
// bytes, so that the result can be returned as a single word on the stack.
// As per the spec, scalars must not have leading zeros.
global decode_rlp_scalar:
    // stack: pos, retdest
    PUSH decode_int_given_len
    // stack: decode_int_given_len, pos, retdest
    SWAP1
    // stack: pos, decode_int_given_len, retdest
    // decode_rlp_string_len will return to decode_int_given_len, at which point
    // the stack will contain (pos', len, retdest), which are the proper args
    // to decode_int_given_len.
    %jump(decode_rlp_string_len)

// Convenience macro to call decode_rlp_scalar and return where we left off.
%macro decode_rlp_scalar
    %stack (pos) -> (pos, %%after)
    %jump(decode_rlp_scalar)
%%after:
%endmacro

// Parse the length of an RLP list from memory.
// Pre stack: pos, retdest
// Post stack: pos', len
global decode_rlp_list_len:
    // stack: pos, retdest
    DUP1
    %mload_current(@SEGMENT_RLP_RAW)
    // stack: first_byte, pos, retdest
    SWAP1
    %increment // increment pos
    SWAP1
    // stack: first_byte, pos', retdest
    // If first_byte is >= 0xf8, it's a > 55 byte list, and
    // first_byte - 0xf7 is the length of the length.
    DUP1
    %gt_const(0xf7) // GT is native while GE is not, so compare to 0xf6 instead
    // stack: first_byte >= 0xf7, first_byte, pos', retdest
    %jumpi(decode_rlp_list_len_big)

    // This is the "small list" case.
    // The list length is first_byte - 0xc0.
    // stack: first_byte, pos', retdest
    %sub_const(0xc0)
    // stack: len, pos', retdest
    %stack (len, pos, retdest) -> (retdest, pos, len)
    JUMP

decode_rlp_list_len_big:
    // The length of the length is first_byte - 0xf7.
    // stack: first_byte, pos', retdest
    %sub_const(0xf7)
    // stack: len_of_len, pos', retdest
    SWAP1
    // stack: pos', len_of_len, retdest
    %jump(decode_int_given_len)

// Convenience macro to call decode_rlp_list_len and return where we left off.
%macro decode_rlp_list_len
    %stack (pos) -> (pos, %%after)
    %jump(decode_rlp_list_len)
%%after:
%endmacro

// Parse an integer of the given length. It is assumed that the integer will
// fit in a single (256-bit) word on the stack.
// Pre stack: pos, len, retdest
// Post stack: pos', int
decode_int_given_len:
    %stack (pos, len, retdest) -> (pos, len, pos, retdest)
    ADD
    // stack: end_pos, pos, retdest
    SWAP1
    // stack: pos, end_pos, retdest
    PUSH 0 // initial accumulator state
    // stack: acc, pos, end_pos, retdest

decode_int_given_len_loop:
    // stack: acc, pos, end_pos, retdest
    DUP3
    DUP3
    EQ
    // stack: pos == end_pos, acc, pos, end_pos, retdest
    %jumpi(decode_int_given_len_finish)
    // stack: acc, pos, end_pos, retdest
    %shl_const(8)
    // stack: acc << 8, pos, end_pos, retdest
    DUP2
    // stack: pos, acc << 8, pos, end_pos, retdest
    %mload_current(@SEGMENT_RLP_RAW)
    // stack: byte, acc << 8, pos, end_pos, retdest
    ADD
    // stack: acc', pos, end_pos, retdest
    // Increment pos.
    SWAP1
    %increment
    SWAP1
    // stack: acc', pos', end_pos, retdest
    %jump(decode_int_given_len_loop)

decode_int_given_len_finish:
    %stack (acc, pos, end_pos, retdest) -> (retdest, pos, acc)
    JUMP
