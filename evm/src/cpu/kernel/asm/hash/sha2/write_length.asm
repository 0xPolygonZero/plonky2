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
    %mstore_kernel_general
    
    // stack: length, last_addr
    SWAP1
    %decrement
    SWAP1
    // stack: length, last_addr - 1
    %shr_const(8)
    // stack: length >> 8, last_addr - 1
    DUP1
    // stack: length >> 8, length >> 8, last_addr - 1
    %and_const(0xff)
    // stack: (length >> 8) % (1 << 8), length >> 8, last_addr - 1
    DUP3
    // stack: last_addr - 1, (length >> 8) % (1 << 8), length >> 8, last_addr - 1
    %mstore_kernel_general
    
    // stack: length >> 8, last_addr - 1
    SWAP1
    %decrement
    SWAP1
    // stack: length >> 8, last_addr - 2
    %shr_const(8)
    // stack: length >> 16, last_addr - 2
    DUP1
    // stack: length >> 16, length >> 16, last_addr - 2
    %and_const(0xff)
    // stack: (length >> 16) % (1 << 8), length >> 16, last_addr - 2
    DUP3
    // stack: last_addr - 2, (length >> 16) % (1 << 8), length >> 16, last_addr - 2
    %mstore_kernel_general

    // stack: length >> 16, last_addr - 2
    SWAP1
    %decrement
    SWAP1
    // stack: length >> 16, last_addr - 3
    %shr_const(8)
    // stack: length >> 24, last_addr - 3
    DUP1
    // stack: length >> 24, length >> 24, last_addr - 3
    %and_const(0xff)
    // stack: (length >> 24) % (1 << 8), length >> 24, last_addr - 3
    DUP3
    // stack: last_addr - 3, (length >> 24) % (1 << 8), length >> 24, last_addr - 3
    %mstore_kernel_general

    // stack: length >> 24, last_addr - 3
    SWAP1
    %decrement
    SWAP1
    // stack: length >> 24, last_addr - 4
    %shr_const(8)
    // stack: length >> 32, last_addr - 4
    DUP1
    // stack: length >> 32, length >> 32, last_addr - 4
    %and_const(0xff)
    // stack: (length >> 32) % (1 << 8), length >> 32, last_addr - 4
    DUP3
    // stack: last_addr - 4, (length >> 32) % (1 << 8), length >> 32, last_addr - 4
    %mstore_kernel_general

    // stack: length >> 32, last_addr - 4
    SWAP1
    %decrement
    SWAP1
    // stack: length >> 32, last_addr - 5
    %shr_const(8)
    // stack: length >> 40, last_addr - 5
    DUP1
    // stack: length >> 40, length >> 40, last_addr - 5
    %and_const(0xff)
    // stack: (length >> 40) % (1 << 8), length >> 40, last_addr - 5
    DUP3
    // stack: last_addr - 5, (length >> 40) % (1 << 8), length >> 40, last_addr - 5
    %mstore_kernel_general

    // stack: length >> 40, last_addr - 5
    SWAP1
    %decrement
    SWAP1
    // stack: length >> 40, last_addr - 6
    %shr_const(8)
    // stack: length >> 48, last_addr - 6
    DUP1
    // stack: length >> 48, length >> 48, last_addr - 6
    %and_const(0xff)
    // stack: (length >> 48) % (1 << 8), length >> 48, last_addr - 6
    DUP3
    // stack: last_addr - 6, (length >> 48) % (1 << 8), length >> 48, last_addr - 6
    %mstore_kernel_general

    // stack: length >> 48, last_addr - 6
    SWAP1
    %decrement
    SWAP1
    // stack: length >> 48, last_addr - 7
    %shr_const(8)
    // stack: length >> 56, last_addr - 7
    DUP1
    // stack: length >> 56, length >> 56, last_addr - 7
    %and_const(0xff)
    // stack: (length >> 56) % (1 << 8), length >> 56, last_addr - 7
    DUP3
    // stack: last_addr - 7, (length >> 56) % (1 << 8), length >> 56, last_addr - 7
    %mstore_kernel_general
    %pop2
    // stack: (empty)
%endmacro
