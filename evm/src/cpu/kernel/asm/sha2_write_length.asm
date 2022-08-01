%macro sha2_write_length
    // stack: length, last_addr
    push 1
    push 8
    shl

    // stack: 1 << 8, length, last_addr
    dup2
    // stack: length, 1 << 8, length, last_addr
    mod
    // stack: length % (1 << 8), length, last_addr
    dup3
    // stack: last_addr, length % (1 << 8), length, last_addr
    %mstore_kernel_general
    
    // stack: 1 << 8, length, last_addr
    dup1
    // stack: 1 << 8, 1 << 8, length, last_addr
    dup2
    // stack: length, 1 << 8, 1 << 8, length, last_addr
    push 8
    shr
    // stack: length >> 8, 1 << 8, 1 << 8, length, last_addr
    mod
    // stack: (length >> 8) % (1 << 8), 1 << 8, length, last_addr
    dup3
    // stack: last_addr, (length >> 8) % (1 << 8), 1 << 8, length, last_addr
    push 1
    swap1
    sub
    // stack: last_addr - 1, (length >> 8) % (1 << 8), 1 << 8, length, last_addr
    %mstore_kernel_general

    // stack: 1 << 8, length, last_addr
    dup1
    // stack: 1 << 8, 1 << 8, length, last_addr
    dup2
    // stack: length, 1 << 8, 1 << 8, length, last_addr
    push 16
    shr
    // stack: length >> 16, 1 << 8, 1 << 8, length, last_addr
    mod
    // stack: (length >> 16) % (1 << 8), 1 << 8, length, last_addr
    dup3
    // stack: last_addr, (length >> 16) % (1 << 8), 1 << 8, length, last_addr
    push 2
    swap1
    sub
    // stack: last_addr - 2, (length >> 16) % (1 << 8), 1 << 8, length, last_addr
    %mstore_kernel_general

    // stack: 1 << 8, length, last_addr
    dup1
    // stack: 1 << 8, 1 << 8, length, last_addr
    dup2
    // stack: length, 1 << 8, 1 << 8, length, last_addr
    push 24
    shr
    // stack: length >> 24, 1 << 8, 1 << 8, length, last_addr
    mod
    // stack: (length >> 24) % (1 << 8), 1 << 8, length, last_addr
    dup3
    // stack: last_addr, (length >> 24) % (1 << 8), 1 << 8, length, last_addr
    push 3
    swap1
    sub
    // stack: last_addr - 1, (length >> 24) % (1 << 8), 1 << 8, length, last_addr
    %mstore_kernel_general

    // stack: 1 << 8, length, last_addr
    dup1
    // stack: 1 << 8, 1 << 8, length, last_addr
    dup2
    // stack: length, 1 << 8, 1 << 8, length, last_addr
    push 32
    shr
    // stack: length >> 32, 1 << 8, 1 << 8, length, last_addr
    mod
    // stack: (length >> 32) % (1 << 8), 1 << 8, length, last_addr
    dup3
    // stack: last_addr, (length >> 32) % (1 << 8), 1 << 8, length, last_addr
    push 4
    swap1
    sub
    // stack: last_addr - 1, (length >> 32) % (1 << 8), 1 << 8, length, last_addr
    %mstore_kernel_general

    // stack: 1 << 8, length, last_addr
    dup1
    // stack: 1 << 8, 1 << 8, length, last_addr
    dup2
    // stack: length, 1 << 8, 1 << 8, length, last_addr
    push 40
    shr
    // stack: length >> 40, 1 << 8, 1 << 8, length, last_addr
    mod
    // stack: (length >> 40) % (1 << 8), 1 << 8, length, last_addr
    dup3
    // stack: last_addr, (length >> 40) % (1 << 8), 1 << 8, length, last_addr
    push 5
    swap1
    sub
    // stack: last_addr - 1, (length >> 40) % (1 << 8), 1 << 8, length, last_addr
    %mstore_kernel_general

    // stack: 1 << 8, length, last_addr
    dup1
    // stack: 1 << 8, 1 << 8, length, last_addr
    dup2
    // stack: length, 1 << 8, 1 << 8, length, last_addr
    push 48
    shr
    // stack: length >> 48, 1 << 8, 1 << 8, length, last_addr
    mod
    // stack: (length >> 48) % (1 << 8), 1 << 8, length, last_addr
    dup3
    // stack: last_addr, (length >> 48) % (1 << 8), 1 << 8, length, last_addr
    push 6
    swap1
    sub
    // stack: last_addr - 1, (length >> 48) % (1 << 8), 1 << 8, length, last_addr
    %mstore_kernel_general

    // stack: 1 << 8, length, last_addr
    dup1
    // stack: 1 << 8, 1 << 8, length, last_addr
    dup2
    // stack: length, 1 << 8, 1 << 8, length, last_addr
    push 56
    shr
    // stack: length >> 56, 1 << 8, 1 << 8, length, last_addr
    mod
    // stack: (length >> 56) % (1 << 8), 1 << 8, length, last_addr
    dup3
    // stack: last_addr, (length >> 56) % (1 << 8), 1 << 8, length, last_addr
    push 7
    swap1
    sub
    // stack: last_addr - 1, (length >> 56) % (1 << 8), 1 << 8, length, last_addr
    %mstore_kernel_general
    %pop3
    // stack: (empty)
%endmacro
