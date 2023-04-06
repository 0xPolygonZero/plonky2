global sha2:
    // stack: virt, num_bytes, retdest
    SWAP1
    // stack: num_bytes, virt, retdest
    DUP2
    // stack: virt, num_bytes, virt, retdest
    %mstore_kernel_general
    // stack: virt, retdest

    PUSH 0 %mload_kernel_general POP
    PUSH 1 %mload_kernel_general POP
    PUSH 2 %mload_kernel_general POP
    PUSH 3 %mload_kernel_general POP
    PUSH 4 %mload_kernel_general POP
    PUSH 5 %mload_kernel_general POP
    PUSH 6 %mload_kernel_general POP
    PUSH 7 %mload_kernel_general POP
    PUSH 8 %mload_kernel_general POP
    PUSH 9 %mload_kernel_general POP
    PUSH 10 %mload_kernel_general POP
    PUSH 11 %mload_kernel_general POP
    PUSH 12 %mload_kernel_general POP
    PUSH 13 %mload_kernel_general POP
    PUSH 14 %mload_kernel_general POP
    PUSH 15 %mload_kernel_general POP
    PUSH 16 %mload_kernel_general POP
    PUSH 17 %mload_kernel_general POP
    PUSH 18 %mload_kernel_general POP
    PUSH 19 %mload_kernel_general POP
    PUSH 20 %mload_kernel_general POP
    PUSH 21 %mload_kernel_general POP
    PUSH 22 %mload_kernel_general POP
    PUSH 23 %mload_kernel_general POP
    PUSH 24 %mload_kernel_general POP
    PUSH 25 %mload_kernel_general POP
    PUSH 26 %mload_kernel_general POP
    PUSH 27 %mload_kernel_general POP
    PUSH 28 %mload_kernel_general POP
    PUSH 29 %mload_kernel_general POP
    PUSH 30 %mload_kernel_general POP
    PUSH 31 %mload_kernel_general POP
    PUSH 32 %mload_kernel_general POP


// Precodition: input is in memory, starting at virt of kernel general segment, of the form
//              num_bytes, x[0], x[1], ..., x[num_bytes - 1]
// Postcodition: output is in memory, starting at 0, of the form
//               num_blocks, block0[0], ..., block0[63], block1[0], ..., blocklast[63]
global sha2_pad:
    // stack: virt, retdest
    %mload_kernel_general
    // stack: num_bytes, retdest
    // STEP 1: append 1
    // insert 128 (= 1 << 7) at x[num_bytes+1]
    // stack: num_bytes, retdest
    PUSH 0x80
    // stack: 128, num_bytes, retdest
    DUP2
    // stack: num_bytes, 128, num_bytes, retdest
    %increment
    // stack: num_bytes+1, 128, num_bytes, retdest
    %mstore_kernel_general
    // stack: num_bytes, retdest
    // STEP 2: calculate num_blocks := (num_bytes+8)//64 + 1
    DUP1
    // stack: num_bytes, num_bytes, retdest
    %add_const(8)
    %div_const(64)
    
    %increment
    // stack: num_blocks = (num_bytes+8)//64 + 1, num_bytes, retdest
    // STEP 3: calculate length := num_bytes*8
    SWAP1
    // stack: num_bytes, num_blocks, retdest
    %mul_const(8)
    // stack: length = num_bytes*8, num_blocks, retdest
    // STEP 4: write length to x[num_blocks*64-7..num_blocks*64]
    DUP2
    // stack: num_blocks, length, num_blocks, retdest
    %mul_const(64)
    // stack: last_addr = num_blocks*64, length, num_blocks, retdest
    %sha2_write_length
    // stack: num_blocks, retdest
    DUP1
    // stack: num_blocks, num_blocks, retdest
    // STEP 5: write num_blocks to x[0]
    PUSH 0
    %mstore_kernel_general
    // stack: num_blocks, retdest
    %message_schedule_addr_from_num_blocks
    %jump(sha2_gen_all_message_schedules)
