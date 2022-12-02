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

// Invert the order of the four bytes in a word.
%macro invert_four_byte_word
    // stack: word
    %mul_const(0x1000000010000000100)
    %and_const(0xff0000ff00ff00000000ff0000)
    %mod_const(0xffffffffffff)
    // stack: word_inverted
%endmacro

// Invert the order of the eight bytes in a Blake word.
%macro invert_bytes_blake_word
    // stack: word
    DUP1
    // stack: word, word
    %and_const(0xffffffff)
    // stack: word_lo, word
    SWAP1
    // stack: word, word_lo
    %shr_const(32)
    // stack: word_hi, word_lo
    %invert_four_byte_word
    // stack: word_hi_inverted, word_lo
    SWAP1
    // stack: word_lo, word_hi_inverted
    %invert_four_byte_word
    // stack: word_lo_inverted, word_hi_inverted
    %shl_const(32)
    OR
    // stack: word_inverted
%endmacro
