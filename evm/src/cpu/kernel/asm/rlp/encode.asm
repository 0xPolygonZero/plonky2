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
    %mstore_current(@SEGMENT_RLP_RAW)
    // stack: pos, retdest
    %add_const(1)
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
    %mstore_current(@SEGMENT_RLP_RAW)
    // stack: len, pos, string, retdest
    SWAP1
    %add_const(1) // increment pos
    // stack: pos, len, string, retdest
    %stack (pos, len, string) -> (@SEGMENT_RLP_RAW, pos, string, len, encode_rlp_fixed_finish, pos, len)
    PUSH 0 // context
    // stack: context, segment, pos, string, len, encode_rlp_fixed, pos, retdest
    %jump(mstore_unpacking)

encode_rlp_fixed_finish:
    // stack: pos, len, retdest
    ADD
    // stack: pos', retdest
    SWAP1
    JUMP

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
    %add_const(1)
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
    SWAP1 %add_const(1)
    // stack: pos', len_of_len, payload_len, retdest
    %stack (pos, len_of_len, payload_len, retdest)
        -> (0, @SEGMENT_RLP_RAW, pos, payload_len, len_of_len,
            encode_rlp_list_prefix_large_done_writing_len,
            pos, len_of_len, retdest)
    %jump(mstore_unpacking)
encode_rlp_list_prefix_large_done_writing_len:
    // stack: pos', len_of_len, retdest
    ADD
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
    DUP1 %add_const(1) // start_len_pos = start_pos + 1
    %stack (start_len_pos, start_pos, len_of_len, payload_len, end_pos, retdest)
        -> (0, @SEGMENT_RLP_RAW, start_len_pos, // context, segment, offset
            payload_len, len_of_len,
            prepend_rlp_list_prefix_big_done_writing_len,
            start_pos, end_pos, retdest)
    %jump(mstore_unpacking)
prepend_rlp_list_prefix_big_done_writing_len:
    // stack: start_pos, end_pos, retdest
    DUP1
    SWAP2
    // stack: end_pos, start_pos, start_pos, retdest
    SUB
    // stack: rlp_len, start_pos, retdest
    %stack (rlp_len, start_pos, retdest) -> (retdest, start_pos, rlp_len)
    JUMP

// Convenience macro to call prepend_rlp_list_prefix and return where we left off.
%macro prepend_rlp_list_prefix
    %stack (start_pos) -> (start_pos, %%after)
    %jump(prepend_rlp_list_prefix)
%%after:
%endmacro

// Get the number of bytes required to represent the given scalar.
// The scalar is assumed to be non-zero, as small scalars like zero should
// have already been handled with the small-scalar encoding.
global num_bytes:
    // stack: x, retdest
    PUSH 0 // i
    // stack: i, x, retdest

num_bytes_loop:
    // stack: i, x, retdest
    // If x[i] != 0, break.
    DUP2 DUP2
    // stack: i, x, i, x, retdest
    BYTE
    // stack: x[i], i, x, retdest
    %jumpi(num_bytes_finish)
    // stack: i, x, retdest

    %add_const(1)
    // stack: i', x, retdest
    %jump(num_bytes_loop)

num_bytes_finish:
    // stack: i, x, retdest
    PUSH 32
    SUB
    %stack (num_bytes, x, retdest) -> (retdest, num_bytes)
    JUMP

// Convenience macro to call num_bytes and return where we left off.
%macro num_bytes
    %stack (x) -> (x, %%after)
    %jump(num_bytes)
%%after:
%endmacro

// Given some scalar, compute the number of bytes used in its RLP encoding,
// including any length prefix.
%macro scalar_rlp_len
    // stack: scalar
    // Since the scalar fits in a word, we can't hit the large (>55 byte)
    // case, so we just check for small vs medium.
    DUP1 %gt_const(0x7f)
    // stack: is_medium, scalar
    %jumpi(%%medium)
    // Small case; result is 1.
    %stack (scalar) -> (1)
%%medium:
    // stack: scalar
    %num_bytes
    // stack: scalar_bytes
    %add_const(1) // Account for the length prefix.
    // stack: rlp_len
%endmacro
