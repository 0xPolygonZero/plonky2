// Load a 64-bit word from kernel general memory.
%macro mload_blake_word
    // stack: offset
    DUP1
    %mload_kernel_general_u32_LE
    // stack: lo, offset
    SWAP1
    // stack: offset, lo
    %add_const(4)
    %mload_kernel_general_u32
    // stack: hi, lo
    %shl_const(32)
    // stack: hi << 32, lo
    OR
    // stack: (hi << 32) | lo
%endmacro
