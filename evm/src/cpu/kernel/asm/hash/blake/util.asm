// Load a 64-bit word from kernel general memory.
%macro mload_blake_word_from_bytes
    // stack: offset
    DUP1
    %mload_kernel_general_u32_LE
    // stack: lo, offset
    SWAP1
    // stack: offset, lo
    %add_const(4)
    %mload_kernel_general_u32_LE
    // stack: hi, lo
    %shl_const(32)
    // stack: hi << 32, lo
    OR
    // stack: (hi << 32) | lo
%endmacro

%macro invert_bytes_blake_word
    // stack: word, ...
    DUP1
    %and_const(0xff)
    %shl_const(56)
    SWAP1
    // stack: word, first_byte, ...
    DUP1
    %shr_const(8)
    %and_const(0xff)
    %shl_const(48)
    SWAP1
    // stack: word, second_byte, first_byte, ...
    DUP1
    %shr_const(16)
    %and_const(0xff)
    %shl_const(40)
    SWAP1
    DUP1
    %shr_const(24)
    %and_const(0xff)
    %shl_const(32)
    SWAP1
    DUP1
    %shr_const(32)
    %and_const(0xff)
    %shl_const(24)
    SWAP1
    DUP1
    %shr_const(40)
    %and_const(0xff)
    %shl_const(16)
    SWAP1
    DUP1
    %shr_const(48)
    %and_const(0xff)
    %shl_const(8)
    SWAP1
    %shr_const(56)
    %and_const(0xff)
    %rep 7
        OR
    %endrep
%endmacro
