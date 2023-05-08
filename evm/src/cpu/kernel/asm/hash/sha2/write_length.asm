%macro sha2_write_length
    // stack: last_addr, length
    SWAP1
    // stack: length, last_addr
    DUP1
    // stack: length, length, last_addr
    %and_const(0xff)
    // stack: length % (1 << 8), length, last_addr
    DUP3
    // stack: last_addr, length % (1 << 8), length, last_addr
    %store_current_general
    
    %rep 7
        // For i = 0 to 6
        // stack: length >> (8 * i), last_addr - i - 1
        SWAP1
        %decrement
        SWAP1
        // stack: length >> (8 * i), last_addr - i - 2
        %div_const(256) // equivalent to %shr_const(8)
        // stack: length >> (8 * (i + 1)), last_addr - i - 2
        DUP1
        // stack: length >> (8 * (i + 1)), length >> (8 * (i + 1)), last_addr - i - 2
        %mod_const(256)
        // stack: (length >> (8 * (i + 1))) % (1 << 8), length >> (8 * (i + 1)), last_addr - i - 2
        DUP3
        // stack: last_addr - i - 2, (length >> (8 * (i + 1))) % (1 << 8), length >> (8 * (i + 1)), last_addr - i - 2
        %store_current_general
    %endrep

    %pop2
    // stack: (empty)
%endmacro
