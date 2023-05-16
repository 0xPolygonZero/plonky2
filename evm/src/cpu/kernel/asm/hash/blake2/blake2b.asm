global blake2b:
    // stack: virt, num_bytes, retdest
    DUP2
    // stack: num_bytes, virt, num_bytes, retdest
    %ceil_div_const(128)
    // stack: num_blocks, virt, num_bytes, retdest
    DUP2
    // stack: virt, num_blocks, virt, num_bytes, retdest
    %mstore_current_general
    // stack: virt, num_bytes, retdest
    %add_const(1)
    %mstore_current_general
    // stack: retdest
    %jump(blake2_compression)
