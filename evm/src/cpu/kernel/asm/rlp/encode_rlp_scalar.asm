// RLP-encode a scalar, i.e. a variable-length integer.
// Pre stack: pos, scalar, retdest
// Post stack: pos
global encode_rlp_scalar:
    // stack: pos, scalar, retdest
    // If scalar > 0x7f, this is the "medium" case.
    DUP2
    %gt_const(0x7f)
    %jumpi(encode_rlp_scalar_medium)

    // Else, if scalar != 0, this is the "small" case, where the value is its own encoding.
    DUP2 %jumpi(encode_rlp_scalar_small)

    // scalar = 0, so BE(scalar) is the empty string, which RLP encodes as a single byte 0x80.
    // stack: pos, scalar, retdest
    %stack (pos, scalar) -> (pos, 0x80, pos)
    %mstore_rlp
    // stack: pos, retdest
    INCREMENT
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

// Doubly-RLP-encode a scalar, i.e. return encode(encode(scalar)).
// Pre stack: pos, scalar, retdest
// Post stack: pos
global doubly_encode_rlp_scalar:
    // stack: pos, scalar, retdest
    // If scalar > 0x7f, this is the "medium" case.
    DUP2
    %gt_const(0x7f)
    %jumpi(doubly_encode_rlp_scalar_medium)

    // Else, if scalar != 0, this is the "small" case, where the value is its own encoding.
    DUP2 %jumpi(encode_rlp_scalar_small)

    // scalar = 0, so BE(scalar) is the empty string, encode(scalar) = 0x80, and encode(encode(scalar)) = 0x8180.
    // stack: pos, scalar, retdest
    %stack (pos, scalar) -> (pos, 0x81, pos, 0x80, pos)
    %mstore_rlp
    // stack: pos, 0x80, pos, retdest
    INCREMENT
    %mstore_rlp
    // stack: pos, retdest
    %add_const(2)
    // stack: pos, retdest
    SWAP1
    JUMP

doubly_encode_rlp_scalar_medium:
    // This is the "medium" case, where
    //     encode(scalar) = [0x80 + len] || BE(scalar)
    // and so
    //     encode(encode(scalar)) = [0x80 + len + 1] || [0x80 + len] || BE(scalar)
    // We first compute the length of the scalar with %num_bytes, then treat the scalar as if it was a
    // fixed-length string with that length.
    // stack: pos, scalar, retdest
    DUP2
    %num_bytes
    // stack: scalar_bytes, pos, scalar, retdest
    %jump(doubly_encode_rlp_fixed)

// The "small" case of RLP-encoding a scalar, where the value is its own encoding.
// This can be used for both for singly encoding or doubly encoding, since encode(encode(x)) = encode(x) = x.
encode_rlp_scalar_small:
    // stack: pos, scalar, retdest
    %stack (pos, scalar) -> (pos, scalar, pos)
    // stack: pos, scalar, pos, retdest
    %mstore_rlp
    // stack: pos, retdest
    INCREMENT
    // stack: pos', retdest
    SWAP1
    JUMP

// Convenience macro to call encode_rlp_scalar and return where we left off.
%macro encode_rlp_scalar
    %stack (pos, scalar) -> (pos, scalar, %%after)
    %jump(encode_rlp_scalar)
%%after:
%endmacro

// Convenience macro to call doubly_encode_rlp_scalar and return where we left off.
%macro doubly_encode_rlp_scalar
    %stack (pos, scalar) -> (pos, scalar, %%after)
    %jump(doubly_encode_rlp_scalar)
%%after:
%endmacro
