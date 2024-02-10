global sha2:
    // stack: virt, num_bytes, retdest
    %build_current_general_address
    // stack: addr, num_bytes, retdest
    DUP1 SWAP2
    // stack: num_bytes, addr, addr, retdest
    MSTORE_GENERAL
    // stack: addr, retdest


// Precondition: input is in memory, starting at addr of kernel general segment, of the form
//              num_bytes, x[0], x[1], ..., x[num_bytes - 1]
// Postcodition: output is in memory, starting at 0, of the form
//               num_blocks, block0[0], ..., block0[63], block1[0], ..., blocklast[63]
global sha2_pad:
    // stack: addr, retdest
    MLOAD_GENERAL
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
    %mstore_current_general
    // stack: num_bytes, retdest
    // STEP 2: calculate num_blocks := (num_bytes+8)//64 + 1
    DUP1
    // stack: num_bytes, num_bytes, retdest
    %add_const(8)
    %shr_const(6)
    
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
    %mstore_current_general_no_offset
    // stack: num_blocks, retdest
    %message_schedule_addr_from_num_blocks
    %jump(sha2_gen_all_message_schedules)
