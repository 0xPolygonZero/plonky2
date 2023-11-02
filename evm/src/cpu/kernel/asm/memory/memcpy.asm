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

%macro memcpy
    %stack (dst: 3, src: 3, count) -> (dst, src, count, %%after)
    %jump(memcpy)
%%after:
%endmacro

// Similar logic to memcpy, but optimized for copying sequences of bytes.
global memcpy_bytes:
    // stack: DST, SRC, count, retdest

    // Handle small case
    DUP7
    // stack: count, DST, SRC, count, retdest
    %lt_const(0x20)
    // stack: count < 32, DST, SRC, count, retdest
    %jumpi(memcpy_bytes_finish)
    
    // We will pack 32 bytes into a U256 from the source, and then unpack it at the destination.
    // Copy the next chunk of bytes.
    PUSH 32
    DUP7
    DUP7
    DUP7
    // stack: SRC, 32, DST, SRC, count, retdest
    MLOAD_32BYTES
    // stack: value, DST, SRC, count, retdest
    DUP4
    DUP4
    DUP4
    // stack: DST, value, DST, SRC, count, retdest
    MSTORE_32BYTES_32
    // stack: new_offset, DST, SRC, count, retdest
    // Increment dst_addr by 32.
    SWAP3
    POP
    // stack: DST, SRC, count, retdest
    // Increment src_addr by 32.
    SWAP5
    %add_const(0x20)
    SWAP5
    // Decrement count by 32.
    SWAP6
    %sub_const(0x20)
    SWAP6

    // Continue the loop.
    %jump(memcpy_bytes)

memcpy_bytes_finish:
    // stack: DST, SRC, count, retdest

    // Handle empty case
    DUP7
    // stack: count, DST, SRC, count, retdest
    ISZERO
    // stack: count == 0, DST, SRC, count, retdest
    %jumpi(memcpy_finish)

    // stack: DST, SRC, count, retdest

    // Copy the last chunk of `count` bytes.
    DUP7
    DUP1
    DUP8
    DUP8
    DUP8
    // stack: SRC, count, count, DST, SRC, count, retdest
    MLOAD_32BYTES
    // stack: value, count, DST, SRC, count, retdest
    DUP5
    DUP5
    DUP5
    // stack: DST, value, count, DST, SRC, count, retdest
    %mstore_unpacking
    // stack: new_offset, DST, SRC, count, retdest
    POP

memcpy_finish:
    // stack: DST, SRC, count, retdest
    %pop7
    // stack: retdest
    JUMP

%macro memcpy_bytes
    %stack (dst: 3, src: 3, count) -> (dst, src, count, %%after)
    %jump(memcpy_bytes)
%%after:
%endmacro
