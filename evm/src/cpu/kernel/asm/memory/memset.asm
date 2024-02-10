// Sets `count` values to 0 at DST.
global memset:
    // stack: DST, count, retdest

    // Handle small case
    DUP2
    // stack: count, DST, count, retdest
    %lt_const(0x21)
    // stack: count <= 32, DST, count, retdest
    %jumpi(memset_finish)

    // stack: DST, count, retdest
    PUSH 0
    SWAP1
    // stack: DST, 0, count, retdest
    MSTORE_32BYTES_32
    // stack: DST', count, retdest
    // Decrement count.
    PUSH 32 DUP3 SUB SWAP2 POP

    // Continue the loop.
    %jump(memset)

memset_finish:
    // stack: DST, final_count, retdest

    // Handle empty case
    DUP2
    // stack: final_count, DST, final_count, retdest
    ISZERO
    // stack: final_count == 0, DST, final_count, retdest
    %jumpi(memset_bytes_empty)

    // stack: DST, final_count, retdest
    DUP2
    PUSH 0
    DUP3
    // stack: DST, 0, final_count, DST, final_count, retdest
    %mstore_unpacking
    // stack: DST, final_count, retdest
    %pop3
    // stack: retdest
    JUMP

memset_bytes_empty:
    // stack: DST, 0, retdest
    %pop2
    // stack: retdest
    JUMP
