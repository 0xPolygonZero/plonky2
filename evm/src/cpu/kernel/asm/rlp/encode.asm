// RLP-encode a scalar, i.e. a variable-length integer.
// Pre stack: pos, scalar
// Post stack: (empty)
global encode_rlp_scalar:
    PANIC // TODO: implement

// RLP-encode a fixed-length 160-bit string. Assumes string < 2^160.
// Pre stack: pos, string
// Post stack: (empty)
global encode_rlp_160:
    PANIC // TODO: implement

// RLP-encode a fixed-length 256-bit string.
// Pre stack: pos, string
// Post stack: (empty)
global encode_rlp_256:
    PANIC // TODO: implement
