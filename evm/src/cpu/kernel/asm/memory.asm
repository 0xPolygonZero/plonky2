// Load a value from the given segment of the current context's memory space.
// Note that main memory values are one byte each, but in general memory values
// can be 256 bits. This macro deals with a single address (unlike MLOAD), so
// if it is used with main memory, it will load a single byte.
%macro mload_current(segment)
    // stack: offset
    PUSH $segment
    // stack: segment, offset
    GET_CONTEXT
    // stack: context, segment, offset
    MLOAD_GENERAL
    // stack: value
%endmacro

// Store a value to the given segment of the current context's memory space.
// Note that main memory values are one byte each, but in general memory values
// can be 256 bits. This macro deals with a single address (unlike MSTORE), so
// if it is used with main memory, it will store a single byte.
%macro mstore_current(segment)
    // stack: offset, value
    PUSH $segment
    // stack: segment, offset, value
    GET_CONTEXT
    // stack: context, segment, offset, value
    MSTORE_GENERAL
    // stack: (empty)
%endmacro

// Load a single byte from kernel code.
%macro mload_kernel_code
    // stack: offset
    PUSH @SEGMENT_CODE
    // stack: segment, offset
    PUSH 0 // kernel has context 0
    // stack: context, segment, offset
    MLOAD_GENERAL
    // stack: value
%endmacro

// Load a big-endian u32, consisting of 4 bytes (c_3, c_2, c_1, c_0),
// from kernel code.
%macro mload_kernel_code_u32
    // stack: offset
    DUP1
    %mload_kernel_code
    // stack: c_3, offset
    %shl_const(8)
    // stack: c_3 << 8, offset
    DUP2
    %add_const(1)
    %mload_kernel_code
    OR
    // stack: (c_3 << 8) | c_2, offset
    %shl_const(8)
    // stack: ((c_3 << 8) | c_2) << 8, offset
    DUP2
    %add_const(2)
    %mload_kernel_code
    OR
    // stack: (((c_3 << 8) | c_2) << 8) | c_1, offset
    %shl_const(8)
    // stack: ((((c_3 << 8) | c_2) << 8) | c_1) << 8, offset
    SWAP1
    %add_const(3)
    %mload_kernel_code
    OR
    // stack: (((((c_3 << 8) | c_2) << 8) | c_1) << 8) | c_0
%endmacro

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
