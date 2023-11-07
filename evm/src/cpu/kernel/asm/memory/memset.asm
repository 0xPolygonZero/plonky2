// Sets `count` values to 0 at
//     DST = (dst_ctx, dst_segment, dst_addr).
// This tuple definition is used for brevity in the stack comments below.
global memset:
    // stack: DST, count, retdest

    // Handle small case
    DUP4
    // stack: count, DST, count, retdest
    %lt_const(0x21)
    // stack: count <= 32, DST, count, retdest
    %jumpi(memset_finish)

    // stack: DST, count, retdest
    PUSH 32
    PUSH 0
    DUP5
    DUP5
    DUP5
    // stack: DST, 0, 32, DST, count, retdest
    MSTORE_32BYTES
    // stack: DST, count, retdest

    // Increment dst_addr.
    SWAP2
    %add_const(0x20)
    SWAP2
    // Decrement count.
    SWAP3
    %sub_const(0x20)
    SWAP3

    // Continue the loop.
    %jump(memset)

memset_finish:
    // stack: DST, final_count, retdest

    // Handle empty case
    DUP4
    // stack: final_count, DST, final_count, retdest
    ISZERO
    // stack: final_count == 0, DST, final_count, retdest
    %jumpi(memset_bytes_empty)

    // stack: DST, final_count, retdest
    DUP4
    PUSH 0
    DUP5
    DUP5
    DUP5
    // stack: DST, 0, final_count, DST, final_count, retdest
    MSTORE_32BYTES
    // stack: DST, final_count, retdest
    %pop4
    // stack: retdest
    JUMP

memset_bytes_empty:
    // stack: DST, 0, retdest
    %pop4
    // stack: retdest
    JUMP
