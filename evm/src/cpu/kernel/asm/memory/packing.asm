// Methods for encoding integers as bytes in memory, as well as the reverse,
// decoding bytes as integers. All big-endian.

global mload_packing:
    // stack: context, segment, offset, len, retdest
    // TODO
    // stack: value

global mstore_unpacking:
    // stack: context, segment, offset, value, len, retdest
    // We will enumerate i in (32 - len)..32.
    // That way BYTE(i, value) will give us the bytes we want.
    DUP5 // len
    PUSH 32
    SUB

mstore_unpacking_loop:
    // stack: i, context, segment, offset, value, len, retdest
    // If i == 32, finish.
    DUP1
    %eq_const(32)
    %jumpi(mstore_unpacking_finish)

    // stack: i, context, segment, offset, value, len, retdest
    DUP5 // value
    DUP2 // i
    BYTE
    // stack: value[i], i, context, segment, offset, value, len, retdest
    DUP5 DUP5 DUP5 // context, segment, offset
    // stack: context, segment, offset, value[i], i, context, segment, offset, value, len, retdest
    MSTORE_GENERAL
    // stack: i, context, segment, offset, value, len, retdest

    // Increment offset.
    SWAP3 %add_const(1) SWAP3
    // Increment i.
    %add_const(1)

    %jump(mstore_unpacking_loop)

mstore_unpacking_finish:
    // stack: i, context, segment, offset, value, len, retdest
    %pop6
    // stack: retdest
    JUMP
