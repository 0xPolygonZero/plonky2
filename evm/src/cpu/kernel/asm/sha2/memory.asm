// Load a single byte from kernel general memory.
%macro mload_kernel_general
    // stack: offset
    PUSH @SEGMENT_KERNEL_GENERAL
    // stack: segment, offset
    PUSH 0 // kernel has context 0
    // stack: context, segment, offset
    MLOAD_GENERAL
    // stack: value
%endmacro

// Load a big-endian u32, consisting of 4 bytes (c_3, c_2, c_1, c_0),
// from kernel general memory.
%macro mload_kernel_general_u32
    // stack: offset
    DUP1
    %mload_kernel_general
    // stack: c_3, offset
    %shl_const(8)
    // stack: c_3 << 8, offset
    DUP2
    %increment
    %mload_kernel_general
    OR
    // stack: (c_3 << 8) | c_2, offset
    %shl_const(8)
    // stack: ((c_3 << 8) | c_2) << 8, offset
    DUP2
    %add_const(2)
    %mload_kernel_general
    OR
    // stack: (((c_3 << 8) | c_2) << 8) | c_1, offset
    %shl_const(8)
    // stack: ((((c_3 << 8) | c_2) << 8) | c_1) << 8, offset
    SWAP1
    %add_const(3)
    %mload_kernel_general
    OR
    // stack: (((((c_3 << 8) | c_2) << 8) | c_1) << 8) | c_0
%endmacro

// Load 256 bits (half of a 512-bit SHA-2 block) from general kernel memory.
%macro mload_kernel_general_u256
    // stack: offset
    DUP1
    %mload_kernel_general_u32
    // stack: c_7, offset
    %shl_const(32)
    // stack: c7 << 32, offset
    DUP2
    %add_const(4)
    %mload_kernel_general_u32
    OR
    // stack: (c_7 << 32) | c_6, offset
    %shl_const(32)
    // stack: ((c_7 << 32) | c_6) << 32, offset
    DUP2
    %add_const(8)
    %mload_kernel_general_u32
    OR
    // stack: (c_7 << 64) | (c_6 << 32) | c_5, offset
    %shl_const(32)
    // stack: ((c_7 << 64) | (c_6 << 32) | c_5) << 32, offset
    DUP2
    %add_const(12)
    %mload_kernel_general_u32
    OR
    // stack: (c_7 << 96) | (c_6 << 64) | (c_5 << 32) | c_4, offset
    %shl_const(32)
    // stack: ((c_7 << 96) | (c_6 << 64) | (c_5 << 32) | c_4) << 32, offset
    DUP2
    %add_const(16)
    %mload_kernel_general_u32
    OR
    // stack: (c_7 << 128) | (c_6 << 96) | (c_5 << 64) | (c_4 << 32) | c_3, offset
    %shl_const(32)
    // stack: ((c_7 << 128) | (c_6 << 96) | (c_5 << 64) | (c_4 << 32) | c_3) << 32, offset
    DUP2
    %add_const(20)
    %mload_kernel_general_u32
    OR
    // stack: (c_7 << 160) | (c_6 << 128) | (c_5 << 96) | (c_4 << 64) | (c_3 << 32) | c_2, offset
    %shl_const(32)
    // stack: ((c_7 << 160) | (c_6 << 128) | (c_5 << 96) | (c_4 << 64) | (c_3 << 32) | c_2) << 32, offset
    DUP2
    %add_const(24)
    %mload_kernel_general_u32
    OR
    // stack: (c_7 << 192) | (c_6 << 160) | (c_5 << 128) | (c_4 << 96) | (c_3 << 64) | (c_2 << 32) | c_1, offset
    %shl_const(32)
    // stack: ((c_7 << 192) | (c_6 << 160) | (c_5 << 128) | (c_4 << 96) | (c_3 << 64) | (c_2 << 32) | c_1) << 32, offset
    DUP2
    %add_const(28)
    %mload_kernel_general_u32
    OR
    // stack: (c_7 << 224) | (c_6 << 192) | (c_5 << 160) | (c_4 << 128) | (c_3 << 96) | (c_2 << 64) | (c_1 << 32) | c_0, offset
    SWAP1
    POP
    // stack: (c_7 << 224) | (c_6 << 192) | (c_5 << 160) | (c_4 << 128) | (c_3 << 96) | (c_2 << 64) | (c_1 << 32) | c_0
%endmacro

// Store a single byte to kernel general memory.
%macro mstore_kernel_general
    // stack: offset, value
    PUSH @SEGMENT_KERNEL_GENERAL
    // stack: segment, offset
    PUSH 0 // kernel has context 0
    // stack: context, segment, offset, value
    MSTORE_GENERAL
%endmacro

// Store a big-endian u32, consisting of 4 bytes (c_3, c_2, c_1, c_0),
// to kernel general memory.
%macro mstore_kernel_general_u32
    // stack: offset, value
    SWAP1
    // stack: value, offset
    PUSH 1
    PUSH 8
    SHL
    // stack: 1 << 8, value, offset
    SWAP1
    // stack: value, 1 << 8, offset
    DUP2
    DUP2
    // stack: value, 1 << 8, value, 1 << 8, offset
    MOD
    // stack: c_0 = value % (1 << 8), value, 1 << 8, offset
    SWAP2
    SWAP1
    // stack: value, 1 << 8, c_0, offset
    PUSH 8
    SHR
    // stack: value >> 8, 1 << 8, c_0, offset
    DUP2
    DUP2
    // stack: value >> 8, 1 << 8, value >> 8, 1 << 8, c_0, offset
    MOD
    // stack: c_1 = (value >> 8) % (1 << 8), value >> 8, 1 << 8, c_0, offset
    SWAP2
    SWAP1
    // stack: value >> 8, 1 << 8, c_1, c_0, offset
    PUSH 8
    SHR
    // stack: value >> 16, 1 << 8, c_1, c_0, offset
    DUP2
    DUP2
    // stack: value >> 16, 1 << 8, value >> 16, 1 << 8, c_1, c_0, offset
    MOD
    // stack: c_2 = (value >> 16) % (1 << 8), value >> 16, 1 << 8, c_1, c_0, offset
    SWAP2
    SWAP1
    // stack: value >> 16, 1 << 8, c_2, c_1, c_0, offset
    PUSH 8
    SHR
    // stack: value >> 24, 1 << 8, c_2, c_1, c_0, offset
    MOD
    // stack: c_3 = (value >> 24) % (1 << 8), c_2, c_1, c_0, offset
    DUP5
    // stack: offset, c_3, c_2, c_1, c_0, offset
    %mstore_kernel_general
    // stack: c_2, c_1, c_0, offset
    DUP4
    // stack: offset, c_2, c_1, c_0, offset
    %add_const(1)
    %mstore_kernel_general
    // stack: c_1, c_0, offset
    DUP3
    // stack: offset, c_1, c_0, offset
    %add_const(2)
    %mstore_kernel_general
    // stack: c_0, offset
    SWAP1
    // stack: offset, c_0
    %add_const(3)
    %mstore_kernel_general
%endmacro
