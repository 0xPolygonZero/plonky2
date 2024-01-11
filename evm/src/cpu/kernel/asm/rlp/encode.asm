// RLP-encode a fixed-length 160 bit (20 byte) string. Assumes string < 2^160.
// Pre stack: rlp_addr, string, retdest
// Post stack: rlp_addr
global encode_rlp_160:
    PUSH 20
    %jump(encode_rlp_fixed)

// Convenience macro to call encode_rlp_160 and return where we left off.
%macro encode_rlp_160
    %stack (rlp_addr, string) -> (rlp_addr, string, %%after)
    %jump(encode_rlp_160)
%%after:
%endmacro

// RLP-encode a fixed-length 256 bit (32 byte) string.
// Pre stack: rlp_addr, string, retdest
// Post stack: rlp_addr
global encode_rlp_256:
    PUSH 32
    %jump(encode_rlp_fixed)

// Convenience macro to call encode_rlp_256 and return where we left off.
%macro encode_rlp_256
    %stack (rlp_addr, string) -> (rlp_addr, string, %%after)
    %jump(encode_rlp_256)
%%after:
%endmacro

// RLP-encode a fixed-length string with the given byte length. Assumes string < 2^(8 * len).
global encode_rlp_fixed:
    // stack: len, rlp_addr, string, retdest
    DUP2
    DUP2
    %add_const(0x80)
    // stack: first_byte, rlp_addr, len, rlp_addr, string, retdest
    MSTORE_GENERAL
    // stack: len, rlp_addr, string, retdest
    SWAP1
    %increment // increment rlp_addr
    // stack: rlp_addr, len, string, retdest
    %stack (rlp_addr, len, string) -> (rlp_addr, string, len, encode_rlp_fixed_finish)
    // stack: rlp_addr, string, len, encode_rlp_fixed_finish, retdest
    %jump(mstore_unpacking)
encode_rlp_fixed_finish:
    // stack: rlp_addr', retdest
    SWAP1
    JUMP

// Doubly-RLP-encode a fixed-length string with the given byte length.
// I.e. writes encode(encode(string). Assumes string < 2^(8 * len).
global doubly_encode_rlp_fixed:
    // stack: len, rlp_addr, string, retdest
    DUP2
    DUP2
    %add_const(0x81)
    // stack: first_byte, rlp_addr, len, rlp_addr, string, retdest
    MSTORE_GENERAL
    // stack: len, rlp_addr, string, retdest
    DUP2 %increment
    DUP2
    %add_const(0x80)
    // stack: second_byte, rlp_addr', len, original_rlp_addr, string, retdest
    MSTORE_GENERAL
    // stack: len, rlp_addr, string, retdest
    SWAP1
    %add_const(2) // advance past the two prefix bytes
    // stack: rlp_addr'', len, string, retdest
    %stack (rlp_addr, len, string) -> (rlp_addr, string, len, encode_rlp_fixed_finish)
    // stack: context, segment, rlp_addr'', string, len, encode_rlp_fixed_finish, retdest
    %jump(mstore_unpacking)

// Writes the RLP prefix for a string of the given length. This does not handle
// the trivial encoding of certain single-byte strings, as handling that would
// require access to the actual string, while this method only accesses its
// length. This method should generally be used only when we know a string
// contains at least two bytes.
//
// Pre stack: rlp_addr, str_len, retdest
// Post stack: rlp_addr'
global encode_rlp_multi_byte_string_prefix:
    // stack: rlp_addr, str_len, retdest
    DUP2 %gt_const(55)
    // stack: str_len > 55, rlp_addr, str_len, retdest
    %jumpi(encode_rlp_multi_byte_string_prefix_large)
    // Medium case; prefix is 0x80 + str_len.
    // stack: rlp_addr, str_len, retdest
    PUSH 0x80
    DUP2
    // stack: rlp_addr, 0x80, rlp_addr, str_len, retdest
    SWAP3 ADD
    // stack: prefix, rlp_addr, rlp_addr, retdest
    MSTORE_GENERAL
    // stack: rlp_addr, retdest
    %increment
    // stack: rlp_addr', retdest
    SWAP1
    JUMP
encode_rlp_multi_byte_string_prefix_large:
    // Large case; prefix is 0xb7 + len_of_len, followed by str_len.
    // stack: rlp_addr, str_len, retdest
    DUP2
    %num_bytes
    // stack: len_of_len, rlp_addr, str_len, retdest
    SWAP1
    DUP1 // rlp_addr
    DUP3 // len_of_len
    %add_const(0xb7)
    // stack: first_byte, rlp_addr, rlp_addr, len_of_len, str_len, retdest
    MSTORE_GENERAL
    // stack: rlp_addr, len_of_len, str_len, retdest
    %increment
    // stack: rlp_addr', len_of_len, str_len, retdest
    %stack (rlp_addr, len_of_len, str_len) -> (rlp_addr, str_len, len_of_len)
    %jump(mstore_unpacking)

%macro encode_rlp_multi_byte_string_prefix
    %stack (rlp_addr, str_len) -> (rlp_addr, str_len, %%after)
    %jump(encode_rlp_multi_byte_string_prefix)
%%after:
%endmacro

// Writes the RLP prefix for a list with the given payload length.
//
// Pre stack: rlp_addr, payload_len, retdest
// Post stack: rlp_addr'
global encode_rlp_list_prefix:
    // stack: rlp_addr, payload_len, retdest
    DUP2 %gt_const(55)
    %jumpi(encode_rlp_list_prefix_large)
    // Small case: prefix is just 0xc0 + length.
    // stack: rlp_addr, payload_len, retdest
    DUP1
    SWAP2
    %add_const(0xc0)
    // stack: prefix, rlp_addr, rlp_addr, retdest
    MSTORE_GENERAL
    // stack: rlp_addr, retdest
    %increment
    SWAP1
    JUMP
encode_rlp_list_prefix_large:
    // Write 0xf7 + len_of_len.
    // stack: rlp_addr, payload_len, retdest
    DUP2 %num_bytes
    // stack: len_of_len, rlp_addr, payload_len, retdest
    DUP2
    DUP2 %add_const(0xf7)
    // stack: first_byte, rlp_addr, len_of_len, rlp_addr, payload_len, retdest
    MSTORE_GENERAL
    // stack: len_of_len, rlp_addr, payload_len, retdest
    SWAP1 %increment
    // stack: rlp_addr', len_of_len, payload_len, retdest
    %stack (rlp_addr, len_of_len, payload_len)
        -> (rlp_addr, payload_len, len_of_len,
            encode_rlp_list_prefix_large_done_writing_len)
    %jump(mstore_unpacking)
encode_rlp_list_prefix_large_done_writing_len:
    // stack: rlp_addr'', retdest
    SWAP1
    JUMP

%macro encode_rlp_list_prefix
    %stack (rlp_addr, payload_len) -> (rlp_addr, payload_len, %%after)
    %jump(encode_rlp_list_prefix)
%%after:
%endmacro

// Given an RLP list payload which starts and ends at the given rlp_address,
// prepend the appropriate RLP list prefix. Returns the updated start rlp_address,
// as well as the length of the RLP data (including the newly-added prefix).
//
// Pre stack: end_rlp_addr, start_rlp_addr, retdest
// Post stack: prefix_start_rlp_addr, rlp_len
global prepend_rlp_list_prefix:
    // stack: end_rlp_addr, start_rlp_addr, retdest
    DUP2 DUP2 SUB // end_rlp_addr - start_rlp_addr
    // stack: payload_len, end_rlp_addr, start_rlp_addr, retdest
    DUP1 %gt_const(55)
    %jumpi(prepend_rlp_list_prefix_big)

    // If we got here, we have a small list, so we prepend 0xc0 + len at rlp_address 8.
    // stack: payload_len, end_rlp_addr, start_rlp_addr, retdest
    PUSH 1 DUP4 SUB // offset of prefix
    DUP2 %add_const(0xc0)
    // stack: prefix_byte, start_rlp_addr-1, payload_len, end_rlp_addr, start_rlp_addr, retdest
    MSTORE_GENERAL
    // stack: payload_len, end_rlp_addr, start_rlp_addr, retdest
    %increment
    // stack: rlp_len, end_rlp_addr, start_rlp_addr, retdest
    SWAP2 %decrement
    // stack: prefix_start_rlp_addr, end_rlp_addr, rlp_len, retdest
    %stack (prefix_start_rlp_addr, end_rlp_addr, rlp_len, retdest) -> (retdest, prefix_start_rlp_addr, rlp_len)
    JUMP

prepend_rlp_list_prefix_big:
    // We have a large list, so we prepend 0xf7 + len_of_len at rlp_address
    //     prefix_start_rlp_addr = start_rlp_addr - 1 - len_of_len
    // followed by the length itself.
    // stack: payload_len, end_rlp_addr, start_rlp_addr, retdest
    DUP1 %num_bytes
    // stack: len_of_len, payload_len, end_rlp_addr, start_rlp_addr, retdest
    DUP1
    PUSH 1 DUP6 SUB // start_rlp_addr - 1
    SUB
    // stack: prefix_start_rlp_addr, len_of_len, payload_len, end_rlp_addr, start_rlp_addr, retdest
    DUP2 %add_const(0xf7) DUP2 %swap_mstore // rlp[prefix_start_rlp_addr] = 0xf7 + len_of_len
    // stack: prefix_start_rlp_addr, len_of_len, payload_len, end_rlp_addr, start_rlp_addr, retdest
    DUP1 %increment // start_len_rlp_addr = prefix_start_rlp_addr + 1
    %stack (start_len_rlp_addr, prefix_start_rlp_addr, len_of_len, payload_len, end_rlp_addr, start_rlp_addr, retdest)
        -> (start_len_rlp_addr, payload_len, len_of_len,
            prepend_rlp_list_prefix_big_done_writing_len,
            prefix_start_rlp_addr, end_rlp_addr, retdest)
    %jump(mstore_unpacking)
prepend_rlp_list_prefix_big_done_writing_len:
    // stack: start_rlp_addr, prefix_start_rlp_addr, end_rlp_addr, retdest
    %stack (start_rlp_addr, prefix_start_rlp_addr, end_rlp_addr)
        -> (end_rlp_addr, prefix_start_rlp_addr, prefix_start_rlp_addr)
    // stack: end_rlp_addr, prefix_start_rlp_addr, prefix_start_rlp_addr, retdest
    SUB
    // stack: rlp_len, prefix_start_rlp_addr, retdest
    %stack (rlp_len, prefix_start_rlp_addr, retdest) -> (retdest, prefix_start_rlp_addr, rlp_len)
    JUMP

// Convenience macro to call prepend_rlp_list_prefix and return where we left off.
%macro prepend_rlp_list_prefix
    %stack (end_rlp_addr, start_rlp_addr) -> (end_rlp_addr, start_rlp_addr, %%after)
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
