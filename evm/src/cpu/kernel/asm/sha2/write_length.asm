%macro sha2_write_length
    // stack: last_addr, length
    swap1
    // stack: length, last_addr
    // TODO: these should be in the other order once SHL implementation is fixed
    push 8
    push 1
    shl

    // stack: 1 << 8, length, last_addr
    dup1
    // stack: 1 << 8, 1 << 8, length, last_addr
    dup3
    // stack: length, 1 << 8, 1 << 8, length, last_addr
    mod
    // stack: length % (1 << 8), 1 << 8, length, last_addr
    dup4
    // stack: last_addr, length % (1 << 8), 1 << 8, length, last_addr
    %mstore_kernel_general
    
    // stack: 1 << 8, length, last_addr
    dup1
    // stack: 1 << 8, 1 << 8, length, last_addr
    dup2
    // stack: length, 1 << 8, 1 << 8, length, last_addr
    push 8
    swap1 // TODO: remove once SHR implementation is fixed
    shr
    // stack: length >> 8, 1 << 8, 1 << 8, length, last_addr
    mod
    // stack: (length >> 8) % (1 << 8), 1 << 8, length, last_addr
    dup4
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
    swap1 // TODO: remove once SHR implementation is fixed
    shr
    // stack: length >> 16, 1 << 8, 1 << 8, length, last_addr
    mod
    // stack: (length >> 16) % (1 << 8), 1 << 8, length, last_addr
    dup4
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
    swap1 // TODO: remove once SHR implementation is fixed
    shr
    // stack: length >> 24, 1 << 8, 1 << 8, length, last_addr
    mod
    // stack: (length >> 24) % (1 << 8), 1 << 8, length, last_addr
    dup4
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
    swap1 // TODO: remove once SHR implementation is fixed
    shr
    // stack: length >> 32, 1 << 8, 1 << 8, length, last_addr
    mod
    // stack: (length >> 32) % (1 << 8), 1 << 8, length, last_addr
    dup4
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
    dup4
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
    swap1 // TODO: remove once SHR implementation is fixed
    shr
    // stack: length >> 48, 1 << 8, 1 << 8, length, last_addr
    mod
    // stack: (length >> 48) % (1 << 8), 1 << 8, length, last_addr
    dup4
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
    swap1 // TODO: remove once SHR implementation is fixed
    shr
    // stack: length >> 56, 1 << 8, 1 << 8, length, last_addr
    mod
    // stack: (length >> 56) % (1 << 8), 1 << 8, length, last_addr
    dup4
    // stack: last_addr, (length >> 56) % (1 << 8), 1 << 8, length, last_addr
    push 7
    swap1
    sub
    // stack: last_addr - 1, (length >> 56) % (1 << 8), 1 << 8, length, last_addr
    %mstore_kernel_general
    %pop3
    // stack: (empty)
%endmacro
