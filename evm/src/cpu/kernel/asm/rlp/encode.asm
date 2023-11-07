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
global encode_rlp_fixed:
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
    %stack (pos, len, string) -> (pos, len, string, encode_rlp_fixed_finish)
    // stack: context, segment, pos, len, string, encode_rlp_fixed_finish, retdest
    %jump(mstore_unpacking_rlp)
encode_rlp_fixed_finish:
    // stack: pos', retdest
    SWAP1
    JUMP

// Doubly-RLP-encode a fixed-length string with the given byte length.
// I.e. writes encode(encode(string). Assumes string < 2^(8 * len).
global doubly_encode_rlp_fixed:
    // stack: len, pos, string, retdest
    DUP1
    %add_const(0x81)
    // stack: first_byte, len, pos, string, retdest
    DUP3
    // stack: pos, first_byte, len, pos, string, retdest
    %mstore_rlp
    // stack: len, pos, string, retdest
    DUP1
    %add_const(0x80)
    // stack: second_byte, len, original_pos, string, retdest
    DUP3 %increment
    // stack: pos', second_byte, len, pos, string, retdest
    %mstore_rlp
    // stack: len, pos, string, retdest
    SWAP1
    %add_const(2) // advance past the two prefix bytes
    // stack: pos'', len, string, retdest
    %stack (pos, len, string) -> (pos, len, string, encode_rlp_fixed_finish)
    // stack: context, segment, pos'', len, string, encode_rlp_fixed_finish, retdest
    %jump(mstore_unpacking_rlp)

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
    %jump(mstore_unpacking_rlp)

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
        -> (pos, len_of_len, payload_len, 
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

// Given an RLP list payload which starts and ends at the given positions,
// prepend the appropriate RLP list prefix. Returns the updated start position,
// as well as the length of the RLP data (including the newly-added prefix).
//
// Pre stack: end_pos, start_pos, retdest
// Post stack: prefix_start_pos, rlp_len
global prepend_rlp_list_prefix:
    // stack: end_pos, start_pos, retdest
    DUP2 DUP2 SUB // end_pos - start_pos
    // stack: payload_len, end_pos, start_pos, retdest
    DUP1 %gt_const(55)
    %jumpi(prepend_rlp_list_prefix_big)

    // If we got here, we have a small list, so we prepend 0xc0 + len at position 8.
    // stack: payload_len, end_pos, start_pos, retdest
    DUP1 %add_const(0xc0)
    // stack: prefix_byte, payload_len, end_pos, start_pos, retdest
    DUP4 %decrement // offset of prefix
    %mstore_rlp
    // stack: payload_len, end_pos, start_pos, retdest
    %increment
    // stack: rlp_len, end_pos, start_pos, retdest
    SWAP2 %decrement
    // stack: prefix_start_pos, end_pos, rlp_len, retdest
    %stack (prefix_start_pos, end_pos, rlp_len, retdest) -> (retdest, prefix_start_pos, rlp_len)
    JUMP

prepend_rlp_list_prefix_big:
    // We have a large list, so we prepend 0xf7 + len_of_len at position
    //     prefix_start_pos = start_pos - 1 - len_of_len
    // followed by the length itself.
    // stack: payload_len, end_pos, start_pos, retdest
    DUP1 %num_bytes
    // stack: len_of_len, payload_len, end_pos, start_pos, retdest
    DUP1
    DUP5 %decrement // start_pos - 1
    SUB
    // stack: prefix_start_pos, len_of_len, payload_len, end_pos, start_pos, retdest
    DUP2 %add_const(0xf7) DUP2 %mstore_rlp // rlp[prefix_start_pos] = 0xf7 + len_of_len
    // stack: prefix_start_pos, len_of_len, payload_len, end_pos, start_pos, retdest
    DUP1 %increment // start_len_pos = prefix_start_pos + 1
    %stack (start_len_pos, prefix_start_pos, len_of_len, payload_len, end_pos, start_pos, retdest)
        -> (start_len_pos, len_of_len, payload_len, 
            prepend_rlp_list_prefix_big_done_writing_len,
            prefix_start_pos, end_pos, retdest)
    %jump(mstore_unpacking_rlp)
prepend_rlp_list_prefix_big_done_writing_len:
    // stack: start_pos, prefix_start_pos, end_pos, retdest
    %stack (start_pos, prefix_start_pos, end_pos)
        -> (end_pos, prefix_start_pos, prefix_start_pos)
    // stack: end_pos, prefix_start_pos, prefix_start_pos, retdest
    SUB
    // stack: rlp_len, prefix_start_pos, retdest
    %stack (rlp_len, prefix_start_pos, retdest) -> (retdest, prefix_start_pos, rlp_len)
    JUMP

// Convenience macro to call prepend_rlp_list_prefix and return where we left off.
%macro prepend_rlp_list_prefix
    %stack (end_pos, start_pos) -> (end_pos, start_pos, %%after)
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
// Pre stack: offset, len, value, retdest
// Post stack: offset'
global mstore_unpacking_rlp:
    // stack: offset, len, value, retdest
    PUSH @SEGMENT_RLP_RAW
    PUSH 0 // context
    %jump(mstore_unpacking)
