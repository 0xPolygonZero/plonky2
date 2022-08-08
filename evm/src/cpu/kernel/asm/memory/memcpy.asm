// Copies `count` values from
//     SRC = (src_ctx, src_segment, src_addr)
// to
//     DST = (dst_ctx, dst_segment, dst_addr).
// These tuple definitions are used for brevity in the stack comments below.
global memcpy:
    JUMPDEST
    // stack: DST, SRC, count, retdest
    DUP7
    // stack: count, DST, SRC, count, retdest
    ISZERO
    // stack: count == 0, DST, SRC, count, retdest
    %jumpi(memcpy_finish)
    // stack: DST, SRC, count, retdest

    // Copy the next value.
    DUP6
    DUP6
    DUP6
    // stack: SRC, DST, SRC, count, retdest
    MLOAD_GENERAL
    // stack: value, DST, SRC, count, retdest
    DUP4
    DUP4
    DUP4
    // stack: DST, value, DST, SRC, count, retdest
    MSTORE_GENERAL
    // stack: DST, SRC, count, retdest

    // Increment dst_addr.
    SWAP2
    %add_const(1)
    SWAP2
    // Increment src_addr.
    SWAP5
    %add_const(1)
    SWAP5
    // Decrement count.
    SWAP6
    %sub_const(1)
    SWAP6

    // Continue the loop.
    %jump(memcpy)

memcpy_finish:
    JUMPDEST
    // stack: DST, SRC, count, retdest
    %pop7
    // stack: retdest
    JUMP
