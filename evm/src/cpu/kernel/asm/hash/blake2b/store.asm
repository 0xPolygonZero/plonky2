global blake2b:
    %jump(blake2b_store)

global blake2b_store:
    // stack: num_bytes, x[0], x[1], ..., x[num_bytes - 1], retdest
    DUP1
    // stack: num_bytes, num_bytes, x[0], x[1], ..., x[num_bytes - 1], retdest
    %add_const(127)
    %div_const(128)
    // stack: num_blocks = ceil(num_bytes / 128), num_bytes, x[0], x[1], ..., x[num_bytes - 1], retdest
    PUSH 0
    // stack: addr=0, num_blocks, num_bytes, x[0], x[1], ..., x[num_bytes - 1], retdest
    %mstore_kernel_general
    // stack: num_bytes, x[0], x[1], ..., x[num_bytes - 1], retdest
    DUP1
    // stack: num_bytes, num_bytes, x[0], x[1], ..., x[num_bytes - 1], retdest
    PUSH 1
    // stack: 1, num_bytes, num_bytes, x[0], x[1], ..., x[num_bytes - 1], retdest
    %mstore_kernel_general
    // stack: num_bytes, x[0], x[1], ..., x[num_bytes - 1], retdest
    PUSH 2
    // stack: addr=2, counter=num_bytes, x[0], x[1], x[2], ... , x[num_bytes-1], retdest
store_loop:
    // stack: addr, counter, x[num_bytes-counter], ... , x[num_bytes-1], retdest
    DUP2
    // stack: counter, addr, counter, x[num_bytes-counter], ... , x[num_bytes-1], retdest
    ISZERO
    %jumpi(store_end)
    // stack: addr, counter, x[num_bytes-counter], ... , x[num_bytes-1], retdest
    %stack (addr, counter, val) -> (addr, val, counter, addr)
    // stack: addr, x[num_bytes-counter], counter, addr,  ... , x[num_bytes-1], retdest
    %mstore_kernel_general
    // stack: counter, addr,  ... , x[num_bytes-1], retdest
    %decrement
    // stack: counter-1, addr,  ... , x[num_bytes-1], retdest
    SWAP1
    // stack: addr, counter-1,  ... , x[num_bytes-1], retdest
    %increment
    // stack: addr+1, counter-1,  ... , x[num_bytes-1], retdest
    %jump(store_loop)
store_end:
    // stack: addr, counter, retdest
    %pop2
    // stack: retdest
    %jump(blake2b_compression)
