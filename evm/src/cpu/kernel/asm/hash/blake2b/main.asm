global blake2b:
    // stack: virt, num_bytes, retdest
    POP
    // stack: num_bytes, retdest
    DUP1
    // stack: num_bytes, num_bytes, retdest
    %add_const(127)
    %div_const(128)
    // stack: num_blocks = ceil(num_bytes / 128), num_bytes, retdest
    %mstore_kernel_general(0)
    // stack: num_bytes, retdest
    %mstore_kernel_general(1)
    // stack: retdest
    %jump(blake2b_compression)
