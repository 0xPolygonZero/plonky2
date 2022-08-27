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

// Load a single byte from user code.
%macro mload_current_code
    // stack: offset
    %mload_current(@SEGMENT_CODE)
    // stack: value
%endmacro

// Load a single value from the given segment of kernel (context 0) memory.
%macro mload_kernel(segment)
    // stack: offset
    PUSH $segment
    // stack: segment, offset
    PUSH 0 // kernel has context 0
    // stack: context, segment, offset
    MLOAD_GENERAL
    // stack: value
%endmacro

// Store a single value from the given segment of kernel (context 0) memory.
%macro mstore_kernel(segment)
    // stack: offset, value
    PUSH $segment
    // stack: segment, offset, value
    PUSH 0 // kernel has context 0
    // stack: context, segment, offset, value
    MSTORE_GENERAL
    // stack: (empty)
%endmacro

// Load a single byte from kernel code.
%macro mload_kernel_code
    // stack: offset
    %mload_kernel(@SEGMENT_CODE)
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

// Store a single byte to kernel code.
%macro mstore_kernel_code
    // stack: offset, value
    %mstore_kernel(@SEGMENT_CODE)
    // stack: (empty)
%endmacro
