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
    %mstore_current_general
    
    %rep 7
        // For i = 0 to 6
        // stack: length >> (8 * i), last_addr - i - 1
        SWAP1
        %decrement
        SWAP1
        // stack: length >> (8 * i), last_addr - i - 2
        %shr_const(8)
        // stack: length >> (8 * (i + 1)), last_addr - i - 2
        PUSH 256
        DUP2
        // stack: length >> (8 * (i + 1)), 256, length >> (8 * (i + 1)), last_addr - i - 2
        MOD
        // stack: (length >> (8 * (i + 1))) % (1 << 8), length >> (8 * (i + 1)), last_addr - i - 2
        DUP3
        // stack: last_addr - i - 2, (length >> (8 * (i + 1))) % (1 << 8), length >> (8 * (i + 1)), last_addr - i - 2
        %mstore_current_general
    %endrep

    %pop2
    // stack: (empty)
%endmacro
