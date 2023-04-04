// Copies `count` values from
//     SRC = (src_ctx, src_segment, src_addr)
// to
//     DST = (dst_ctx, dst_segment, dst_addr).
// These tuple definitions are used for brevity in the stack comments below.
global memcpy:
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
    %increment
    SWAP2
    // Increment src_addr.
    SWAP5
    %increment
    SWAP5
    // Decrement count.
    SWAP6
    %decrement
    SWAP6

    // Continue the loop.
    %jump(memcpy)

memcpy_finish:
    // stack: DST, SRC, count, retdest
    %pop7
    // stack: retdest
    JUMP

// Copies zeros to `[addr..addr+count]` in the given context and segment.
global zerocpy:
    // stack: context, segment, addr, count, retdest
    %stack (context, segment, addr, count, retdest) -> (0, count, context, segment, addr, retdest)
zerocpy_loop:
    // stack: i, count, context, segment, addr, retdest
    DUP2 DUP2 EQ %jumpi(zerocpy_finish)
    %stack (i, count, context, segment, addr, retdest) -> (context, segment, addr, 0, addr, i, count, context, segment, retdest)
    MSTORE_GENERAL
    // stack: addr, i, count, context, segment, retdest
    %increment
    SWAP1
    %increment
    %stack (i, addr, count, context, segment, retdest) -> (i, count, context, segment, addr, retdest)
    %jump(zerocpy_loop)

zerocpy_finish:
    // stack: i, count, context, segment, addr, retdest
    %pop5
    JUMP
