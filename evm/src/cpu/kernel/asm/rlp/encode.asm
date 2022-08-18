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
    GET_CONTEXT
    // stack: context, segment, pos, string, len, encode_rlp_fixed, pos, retdest
    %jump(mstore_unpacking)

encode_rlp_fixed_finish:
    // stack: pos, len, retdest
    ADD
    // stack: pos', retdest
    SWAP1
    JUMP

// Get the number of bytes required to represent the given scalar.
// The scalar is assumed to be non-zero, as small scalars like zero should
// have already been handled with the small-scalar encoding.
num_bytes:
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
