// Sets `count` values to `value` at
//     DST = (dst_ctx, dst_segment, dst_addr).
// This tuple definition is used for brevity in the stack comments below.
global memset:
    // stack: DST, value, count, retdest
    DUP5
    // stack: count, DST, value, count, retdest
    ISZERO
    // stack: count == 0, DST, value, count, retdest
    %jumpi(memset_finish)
    // stack: DST, value, count, retdest
    
    DUP3
    DUP3
    DUP3
    // stack: DST, DST, value, count, retdest
    DUP7
    // stack: value, DST, DST, value, count, retdest
    MSTORE_GENERAL
    // stack: DST, value, count, retdest

    // Increment dst_addr.
    SWAP2
    %increment
    SWAP2
    // Decrement count.
    SWAP4
    %decrement
    SWAP4

    // Continue the loop.
    %jump(memset)

memset_finish:
    // stack: DST, value, count, retdest
    %pop5
    // stack: retdest
    JUMP
