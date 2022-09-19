%macro sha2_write_length
    // stack: last_addr, length
    SWAP1
    // stack: length, last_addr
    PUSH 1
    PUSH 8
    SHL

    // stack: 1 << 8, length, last_addr
    DUP1
    // stack: 1 << 8, 1 << 8, length, last_addr
    DUP3
    // stack: length, 1 << 8, 1 << 8, length, last_addr
    MOD
    // stack: length % (1 << 8), 1 << 8, length, last_addr
    DUP4
    // stack: last_addr, length % (1 << 8), 1 << 8, length, last_addr
    %mstore_kernel_general
    
    // stack: 1 << 8, length, last_addr
    DUP1
    // stack: 1 << 8, 1 << 8, length, last_addr
    DUP3
    // stack: length, 1 << 8, 1 << 8, length, last_addr
    PUSH 8
    SHR
    // stack: length >> 8, 1 << 8, 1 << 8, length, last_addr
    MOD
    // stack: (length >> 8) % (1 << 8), 1 << 8, length, last_addr
    DUP4
    // stack: last_addr, (length >> 8) % (1 << 8), 1 << 8, length, last_addr
    PUSH 1
    SWAP1
    SUB
    // stack: last_addr - 1, (length >> 8) % (1 << 8), 1 << 8, length, last_addr
    %mstore_kernel_general

    // stack: 1 << 8, length, last_addr
    DUP1
    // stack: 1 << 8, 1 << 8, length, last_addr
    DUP3
    // stack: length, 1 << 8, 1 << 8, length, last_addr
    PUSH 16
    SHR
    // stack: length >> 16, 1 << 8, 1 << 8, length, last_addr
    MOD
    // stack: (length >> 16) % (1 << 8), 1 << 8, length, last_addr
    DUP4
    // stack: last_addr, (length >> 16) % (1 << 8), 1 << 8, length, last_addr
    PUSH 2
    SWAP1
    SUB
    // stack: last_addr - 2, (length >> 16) % (1 << 8), 1 << 8, length, last_addr
    %mstore_kernel_general

    // stack: 1 << 8, length, last_addr
    DUP1
    // stack: 1 << 8, 1 << 8, length, last_addr
    DUP3
    // stack: length, 1 << 8, 1 << 8, length, last_addr
    PUSH 24
    SHR
    // stack: length >> 24, 1 << 8, 1 << 8, length, last_addr
    MOD
    // stack: (length >> 24) % (1 << 8), 1 << 8, length, last_addr
    DUP4
    // stack: last_addr, (length >> 24) % (1 << 8), 1 << 8, length, last_addr
    PUSH 3
    SWAP1
    SUB
    // stack: last_addr - 3, (length >> 24) % (1 << 8), 1 << 8, length, last_addr
    %mstore_kernel_general

    // stack: 1 << 8, length, last_addr
    DUP1
    // stack: 1 << 8, 1 << 8, length, last_addr
    DUP3
    // stack: length, 1 << 8, 1 << 8, length, last_addr
    PUSH 32
    SHR
    // stack: length >> 32, 1 << 8, 1 << 8, length, last_addr
    MOD
    // stack: (length >> 32) % (1 << 8), 1 << 8, length, last_addr
    DUP4
    // stack: last_addr, (length >> 32) % (1 << 8), 1 << 8, length, last_addr
    PUSH 4
    SWAP1
    SUB
    // stack: last_addr - 4, (length >> 32) % (1 << 8), 1 << 8, length, last_addr
    %mstore_kernel_general

    // stack: 1 << 8, length, last_addr
    DUP1
    // stack: 1 << 8, 1 << 8, length, last_addr
    DUP3
    // stack: length, 1 << 8, 1 << 8, length, last_addr
    PUSH 40
    SHR
    // stack: length >> 40, 1 << 8, 1 << 8, length, last_addr
    MOD
    // stack: (length >> 40) % (1 << 8), 1 << 8, length, last_addr
    DUP4
    // stack: last_addr, (length >> 40) % (1 << 8), 1 << 8, length, last_addr
    PUSH 5
    SWAP1
    SUB
    // stack: last_addr - 5, (length >> 40) % (1 << 8), 1 << 8, length, last_addr
    %mstore_kernel_general

    // stack: 1 << 8, length, last_addr
    DUP1
    // stack: 1 << 8, 1 << 8, length, last_addr
    DUP3
    // stack: length, 1 << 8, 1 << 8, length, last_addr
    PUSH 48
    SHR
    // stack: length >> 48, 1 << 8, 1 << 8, length, last_addr
    MOD
    // stack: (length >> 48) % (1 << 8), 1 << 8, length, last_addr
    DUP4
    // stack: last_addr, (length >> 48) % (1 << 8), 1 << 8, length, last_addr
    PUSH 6
    SWAP1
    SUB
    // stack: last_addr - 6, (length >> 48) % (1 << 8), 1 << 8, length, last_addr
    %mstore_kernel_general

    // stack: 1 << 8, length, last_addr
    DUP1
    // stack: 1 << 8, 1 << 8, length, last_addr
    DUP3
    // stack: length, 1 << 8, 1 << 8, length, last_addr
    PUSH 56
    SHR
    // stack: length >> 56, 1 << 8, 1 << 8, length, last_addr
    MOD
    // stack: (length >> 56) % (1 << 8), 1 << 8, length, last_addr
    DUP4
    // stack: last_addr, (length >> 56) % (1 << 8), 1 << 8, length, last_addr
    PUSH 7
    SWAP1
    SUB
    // stack: last_addr - 7, (length >> 56) % (1 << 8), 1 << 8, length, last_addr
    %mstore_kernel_general
    %pop3
    // stack: (empty)
%endmacro
