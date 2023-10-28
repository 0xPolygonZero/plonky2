// Copies `count` values from
//     SRC = (src_ctx, src_segment, src_addr)
// to
//     DST = (dst_ctx, dst_segment, dst_addr).
// These tuple definitions are used for brevity in the stack comments below.
global memcpy:
    // stack: DST, SRC, count, retdest
    DUP3
    // stack: count, DST, SRC, count, retdest
    ISZERO
    // stack: count == 0, DST, SRC, count, retdest
    %jumpi(memcpy_finish)
    // stack: DST, SRC, count, retdest

    // Copy the next value.
    DUP2
    // stack: SRC, DST, SRC, count, retdest
    MLOAD_GENERAL
    // stack: value, DST, SRC, count, retdest
    DUP2
    // stack: DST, value, DST, SRC, count, retdest
    MSTORE_GENERAL
    // stack: DST, SRC, count, retdest

    // Increment dst_addr.
    %increment
    // Increment src_addr.
    SWAP1
    %increment
    SWAP1
    // Decrement count.
    PUSH 1 DUP4 SUB SWAP3 POP

    // Continue the loop.
    %jump(memcpy)

%macro memcpy
    %stack (dst, src, count) -> (dst, src, count, %%after)
    %jump(memcpy)
%%after:
%endmacro

// Similar logic to memcpy, but optimized for copying sequences of bytes.
global memcpy_bytes:
    // stack: DST, SRC, count, retdest

    // Handle small case
    DUP3
    // stack: count, DST, SRC, count, retdest
    %lt_const(0x21)
    // stack: count <= 32, DST, SRC, count, retdest
    %jumpi(memcpy_bytes_finish)
    
    // We will pack 32 bytes into a U256 from the source, and then unpack it at the destination.
    // Copy the next chunk of bytes.
    // stack: DST, SRC, count, retdest
    PUSH 32
    DUP1
    DUP4
    // stack: SRC, 32, 32, DST, SRC, count, retdest
    MLOAD_32BYTES
    // stack: value, 32, DST, SRC, count, retdest
    DUP3
    // stack: DST, value, 32, DST, SRC, count, retdest
    MSTORE_32BYTES
    // stack: DST, SRC, count, retdest

    // Increment dst_addr by 32.
    %add_const(0x20)
    // Increment src_addr by 32.
    SWAP1
    %add_const(0x20)
    SWAP1
    // Decrement count by 32.
    PUSH 32 DUP4 SUB SWAP3 POP

    // Continue the loop.
    %jump(memcpy_bytes)

memcpy_bytes_finish:
    // stack: DST, SRC, count, retdest

    // Handle empty case
    DUP3
    // stack: count, DST, SRC, count, retdest
    ISZERO
    // stack: count == 0, DST, SRC, count, retdest
    %jumpi(memcpy_finish)

    // stack: DST, SRC, count, retdest

    // Copy the last chunk of `count` bytes.
    DUP3
    DUP1
    DUP4
    // stack: SRC, count, count, DST, SRC, count, retdest
    MLOAD_32BYTES
    // stack: value, count, DST, SRC, count, retdest
    DUP3
    // stack: DST, value, count, DST, SRC, count, retdest
    MSTORE_32BYTES
    // stack: DST, SRC, count, retdest

memcpy_finish:
    // stack: DST, SRC, count, retdest
    %pop3
    // stack: retdest
    JUMP

%macro memcpy_bytes
    %stack (dst, src, count) -> (dst, src, count, %%after)
    %jump(memcpy_bytes)
%%after:
%endmacro
