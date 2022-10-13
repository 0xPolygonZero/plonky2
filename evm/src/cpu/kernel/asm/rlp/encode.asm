// RLP-encode a scalar, i.e. a variable-length integer.
// Pre stack: pos, scalar, retdest
// Post stack: pos
global encode_rlp_scalar:
    // stack: pos, scalar, retdest
    // If scalar > 0x7f, this is the "medium" case.
    DUP2
    %gt_const(0x7f)
    %jumpi(encode_rlp_scalar_medium)

    // This is the "small" case, where the value is its own encoding.
    // stack: pos, scalar, retdest
    %stack (pos, scalar) -> (pos, scalar, pos)
    // stack: pos, scalar, pos, retdest
    %mstore_rlp
    // stack: pos, retdest
    %increment
    // stack: pos', retdest
    SWAP1
    JUMP

encode_rlp_scalar_medium:
    // This is the "medium" case, where we write 0x80 + len followed by the
    // (big-endian) scalar bytes. We first compute the minimal number of bytes
    // needed to represent this scalar, then treat it as if it was a fixed-
    // length string with that length.
    // stack: pos, scalar, retdest
    DUP2
    %num_bytes
    // stack: scalar_bytes, pos, scalar, retdest
    %jump(encode_rlp_fixed)

// Convenience macro to call encode_rlp_scalar and return where we left off.
%macro encode_rlp_scalar
    %stack (pos, scalar) -> (pos, scalar, %%after)
    %jump(encode_rlp_scalar)
%%after:
%endmacro

// RLP-encode a fixed-length 160 bit (20 byte) string. Assumes string < 2^160.
// Pre stack: pos, string, retdest
// Post stack: pos
global encode_rlp_160:
    PUSH 20
    %jump(encode_rlp_fixed)

// Convenience macro to call encode_rlp_160 and return where we left off.
%macro encode_rlp_160
    %stack (pos, string) -> (pos, string, %%after)
    %jump(encode_rlp_160)
%%after:
%endmacro

// RLP-encode a fixed-length 256 bit (32 byte) string.
// Pre stack: pos, string, retdest
// Post stack: pos
global encode_rlp_256:
    PUSH 32
    %jump(encode_rlp_fixed)

// Convenience macro to call encode_rlp_256 and return where we left off.
%macro encode_rlp_256
    %stack (pos, string) -> (pos, string, %%after)
    %jump(encode_rlp_256)
%%after:
%endmacro

// RLP-encode a fixed-length string with the given byte length. Assumes string < 2^(8 * len).
encode_rlp_fixed:
    // stack: len, pos, string, retdest
    DUP1
    %add_const(0x80)
    // stack: first_byte, len, pos, string, retdest
    DUP3
    // stack: pos, first_byte, len, pos, string, retdest
    %mstore_rlp
    // stack: len, pos, string, retdest
    SWAP1
    %increment // increment pos
    // stack: pos, len, string, retdest
    %stack (pos, len, string) -> (pos, string, len, encode_rlp_fixed_finish)
    // stack: context, segment, pos, string, len, encode_rlp_fixed_finish, retdest
    %jump(mstore_unpacking_rlp)
encode_rlp_fixed_finish:
    // stack: pos', retdest
    SWAP1
    JUMP

// Writes the RLP prefix for a string of the given length. This does not handle
// the trivial encoding of certain single-byte strings, as handling that would
// require access to the actual string, while this method only accesses its
// length. This method should generally be used only when we know a string
// contains at least two bytes.
//
// Pre stack: pos, str_len, retdest
// Post stack: pos'
global encode_rlp_multi_byte_string_prefix:
    // stack: pos, str_len, retdest
    DUP2 %gt_const(55)
    // stack: str_len > 55, pos, str_len, retdest
    %jumpi(encode_rlp_multi_byte_string_prefix_large)
    // Medium case; prefix is 0x80 + str_len.
    // stack: pos, str_len, retdest
    SWAP1 %add_const(0x80)
    // stack: prefix, pos, retdest
    DUP2
    // stack: pos, prefix, pos, retdest
    %mstore_rlp
    // stack: pos, retdest
    %increment
    // stack: pos', retdest
    SWAP1
    JUMP
encode_rlp_multi_byte_string_prefix_large:
    // Large case; prefix is 0xb7 + len_of_len, followed by str_len.
    // stack: pos, str_len, retdest
    DUP2
    %num_bytes
    // stack: len_of_len, pos, str_len, retdest
    SWAP1
    DUP2 // len_of_len
    %add_const(0xb7)
    // stack: first_byte, pos, len_of_len, str_len, retdest
    DUP2
    // stack: pos, first_byte, pos, len_of_len, str_len, retdest
    %mstore_rlp
    // stack: pos, len_of_len, str_len, retdest
    %increment
    // stack: pos', len_of_len, str_len, retdest
    %stack (pos, len_of_len, str_len)
        -> (pos, str_len, len_of_len,
            encode_rlp_multi_byte_string_prefix_large_done_writing_len)
    %jump(mstore_unpacking_rlp)
encode_rlp_multi_byte_string_prefix_large_done_writing_len:
    // stack: pos'', retdest
    SWAP1
    JUMP

%macro encode_rlp_multi_byte_string_prefix
    %stack (pos, str_len) -> (pos, str_len, %%after)
    %jump(encode_rlp_multi_byte_string_prefix)
%%after:
%endmacro

// Writes the RLP prefix for a list with the given payload length.
//
// Pre stack: pos, payload_len, retdest
// Post stack: pos'
global encode_rlp_list_prefix:
    // stack: pos, payload_len, retdest
    DUP2 %gt_const(55)
    %jumpi(encode_rlp_list_prefix_large)
    // Small case: prefix is just 0xc0 + length.
    // stack: pos, payload_len, retdest
    SWAP1
    %add_const(0xc0)
    // stack: prefix, pos, retdest
    DUP2
    // stack: pos, prefix, pos, retdest
    %mstore_rlp
    // stack: pos, retdest
    %increment
    SWAP1
    JUMP
encode_rlp_list_prefix_large:
    // Write 0xf7 + len_of_len.
    // stack: pos, payload_len, retdest
    DUP2 %num_bytes
    // stack: len_of_len, pos, payload_len, retdest
    DUP1 %add_const(0xf7)
    // stack: first_byte, len_of_len, pos, payload_len, retdest
    DUP3 // pos
    %mstore_rlp
    // stack: len_of_len, pos, payload_len, retdest
    SWAP1 %increment
    // stack: pos', len_of_len, payload_len, retdest
    %stack (pos, len_of_len, payload_len)
        -> (pos, payload_len, len_of_len,
            encode_rlp_list_prefix_large_done_writing_len)
    %jump(mstore_unpacking_rlp)
encode_rlp_list_prefix_large_done_writing_len:
    // stack: pos'', retdest
    SWAP1
    JUMP

%macro encode_rlp_list_prefix
    %stack (pos, payload_len) -> (pos, payload_len, %%after)
    %jump(encode_rlp_list_prefix)
%%after:
%endmacro

// Given an RLP list payload which starts at position 9 and ends at the given
// position, prepend the appropriate RLP list prefix. Returns the updated start
// position, as well as the length of the RLP data (including the newly-added
// prefix).
//
// (We sometimes start list payloads at position 9 because 9 is the length of
// the longest possible RLP list prefix.)
//
// Pre stack: end_pos, retdest
// Post stack: start_pos, rlp_len
global prepend_rlp_list_prefix:
    // stack: end_pos, retdest
    // Since the list payload starts at position 9, payload_len = end_pos - 9.
    PUSH 9 DUP2 SUB
    // stack: payload_len, end_pos, retdest
    DUP1 %gt_const(55)
    %jumpi(prepend_rlp_list_prefix_big)

    // If we got here, we have a small list, so we prepend 0xc0 + len at position 8.
    // stack: payload_len, end_pos, retdest
    %add_const(0xc0)
    // stack: prefix_byte, end_pos, retdest
    PUSH 8 // offset
    %mstore_rlp
    // stack: end_pos, retdest
    %sub_const(8)
    // stack: rlp_len, retdest
    PUSH 8 // start_pos
    %stack (start_pos, rlp_len, retdest) -> (retdest, start_pos, rlp_len)
    JUMP

prepend_rlp_list_prefix_big:
    // We have a large list, so we prepend 0xf7 + len_of_len at position
    // 8 - len_of_len, followed by the length itself.
    // stack: payload_len, end_pos, retdest
    DUP1 %num_bytes
    // stack: len_of_len, payload_len, end_pos, retdest
    DUP1
    PUSH 8
    SUB
    // stack: start_pos, len_of_len, payload_len, end_pos, retdest
    DUP2 %add_const(0xf7) DUP2 %mstore_rlp // rlp[start_pos] = 0xf7 + len_of_len
    DUP1 %increment // start_len_pos = start_pos + 1
    %stack (start_len_pos, start_pos, len_of_len, payload_len, end_pos, retdest)
        -> (start_len_pos, payload_len, len_of_len,
            prepend_rlp_list_prefix_big_done_writing_len,
            start_pos, end_pos, retdest)
    %jump(mstore_unpacking_rlp)
prepend_rlp_list_prefix_big_done_writing_len:
    // stack: 9, start_pos, end_pos, retdest
    %stack (_9, start_pos, end_pos) -> (end_pos, start_pos, start_pos)
    // stack: end_pos, start_pos, start_pos, retdest
    SUB
    // stack: rlp_len, start_pos, retdest
    %stack (rlp_len, start_pos, retdest) -> (retdest, start_pos, rlp_len)
    JUMP

// Convenience macro to call prepend_rlp_list_prefix and return where we left off.
%macro prepend_rlp_list_prefix
    %stack (end_pos) -> (end_pos, %%after)
    %jump(prepend_rlp_list_prefix)
%%after:
%endmacro

// Given some scalar, compute the number of bytes used in its RLP encoding,
// including any length prefix.
%macro rlp_scalar_len
    // stack: scalar
    // Since the scalar fits in a word, we can't hit the large (>55 byte)
    // case, so we just check for small vs medium.
    DUP1 %gt_const(0x7f)
    // stack: is_medium, scalar
    %jumpi(%%medium)
    // Small case; result is 1.
    %stack (scalar) -> (1)
    %jump(%%finish)
%%medium:
    // stack: scalar
    %num_bytes
    // stack: scalar_bytes
    %increment // Account for the length prefix.
    // stack: rlp_len
%%finish:
%endmacro

// Given some list with the given payload length, compute the number of bytes
// used in its RLP encoding, including the list prefix.
%macro rlp_list_len
    // stack: payload_len
    DUP1 %gt_const(55)
    // stack: is_large, payload_len
    %jumpi(%%large)
    // Small case; prefix is a single byte.
    %increment
    // stack: 1 + payload_len
    %jump(%%finish)
%%large:
    // Prefix is 1 byte containing len_of_len, followed by len_of_len bytes containing len.
    // stack: payload_len
    DUP1 %num_bytes
    // stack: len_of_len, payload_len
    %increment
    // stack: prefix_len, payload_len
    ADD
%%finish:
%endmacro

// Like mstore_unpacking, but specifically for the RLP segment.
// Pre stack: offset, value, len, retdest
// Post stack: offset'
global mstore_unpacking_rlp:
    // stack: offset, value, len, retdest
    PUSH @SEGMENT_RLP_RAW
    PUSH 0 // context
    %jump(mstore_unpacking)
