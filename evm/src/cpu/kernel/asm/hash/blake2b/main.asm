global blake2b:
    // stack: virt, num_bytes, retdest
    DUP2
    // stack: num_bytes, virt, num_bytes, retdest
    %add_const(127)
    %div_const(128)
    // stack: num_blocks = ceil(num_bytes / 128), virt, num_bytes, retdest
    DUP2
    // stack: virt, num_blocks, virt, num_bytes, retdest
    %mstore_kernel_general
    // stack: virt, num_bytes, retdest
    %add_const(1)
    %mstore_kernel_general
    // stack: retdest
    %jump(blake2b_compression)
