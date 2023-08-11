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
    DUP3
    DUP3
    DUP3

    // Copy the next value
    // stack: DST, DST, SRC, count, retdest
    DUP9
    DUP9
    DUP9
    // stack: SRC, DST, DST, SRC, count, retdest
    MLOAD_GENERAL
    // stack: value, DST, DST, SRC, count, retdest
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

%macro memcpy
    %stack (dst: 3, src: 3, count) -> (dst, src, count, %%after)
    %jump(memcpy)
%%after:
%endmacro
