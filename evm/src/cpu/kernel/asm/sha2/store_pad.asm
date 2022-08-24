global sha2_store:
    JUMPDEST
    // stack: num_bytes, x[0], x[1], ..., x[num_bytes - 1], retdest
    dup1
    // stack: num_bytes, num_bytes, x[0], x[1], ..., x[num_bytes - 1], retdest
    push 0
    // stack: addr=0, num_bytes, num_bytes, x[0], x[1], ..., x[num_bytes - 1], retdest
    %mstore_kernel_general
    // stack: num_bytes, x[0], x[1], ..., x[num_bytes - 1], retdest
    push 1
    // stack: addr=1, counter=num_bytes, x[0], x[1], x[2], ... , x[num_bytes-1], retdest
sha2_store_loop:
    JUMPDEST
    // stack: addr, counter, x[num_bytes-counter], ... , x[num_bytes-1], retdest
    dup1
    // stack: addr, addr, counter, x[num_bytes-counter], ... , x[num_bytes-1], retdest
    swap3
    // stack: x[num_bytes-counter], addr, counter, addr,  ... , x[num_bytes-1], retdest
    swap1
    // stack: addr, x[num_bytes-counter], counter, addr,  ... , x[num_bytes-1], retdest
    %mstore_kernel_general
    // stack: counter, addr,  ... , x[num_bytes-1], retdest
    %decrement
    // stack: counter-1, addr,  ... , x[num_bytes-1], retdest
    dup1
    // stack: counter-1, counter-1, addr,  ... , x[num_bytes-1], retdest
    iszero
    %jumpi(sha2_store_end)
    // stack: counter-1, addr,  ... , x[num_bytes-1], retdest
    swap1
    // stack: addr, counter-1,  ... , x[num_bytes-1], retdest
    %increment
    // stack: addr+1, counter-1,  ... , x[num_bytes-1], retdest
    %jump(sha2_store_loop)
sha2_store_end:
    JUMPDEST
    // stack: counter=0, addr, retdest
    %pop2
    // stack: retdest
    %jump(sha2_pad)

// Precodition: input is in memory, starting at 0 of kernel general segment, of the form
//              num_bytes, x[0], x[1], ..., x[num_bytes - 1]
// Postcodition: output is in memory, starting at 0, of the form
//               num_blocks, block0[0], ..., block0[63], block1[0], ..., blocklast[63]
global sha2_pad:
    JUMPDEST
    // stack: retdest
    push 0
    %mload_kernel_general
    // stack: num_bytes, retdest
    // STEP 1: append 1
    // insert 128 (= 1 << 7) at x[num_bytes+1]
    // stack: num_bytes, retdest
    push 1
    push 7
    shl
    // stack: 128, num_bytes, retdest
    dup2
    // stack: num_bytes, 128, num_bytes, retdest
    %increment
    // stack: num_bytes+1, 128, num_bytes, retdest
    %mstore_kernel_general
    // stack: num_bytes, retdest
    // STEP 2: calculate num_blocks := (num_bytes+8)//64 + 1
    dup1
    // stack: num_bytes, num_bytes, retdest
    %add_const(8)
    %div_const(64)
    
    %increment
    // stack: num_blocks = (num_bytes+8)//64 + 1, num_bytes, retdest
    // STEP 3: calculate length := num_bytes*8
    swap1
    // stack: num_bytes, num_blocks, retdest
    push 8
    mul
    // stack: length = num_bytes*8, num_blocks, retdest
    // STEP 4: write length to x[num_blocks*64-7..num_blocks*64]
    dup2
    // stack: num_blocks, length, num_blocks, retdest
    push 64
    mul
    // stack: last_addr = num_blocks*64, length, num_blocks, retdest
    %sha2_write_length
    // stack: num_blocks, retdest
    dup1
    // stack: num_blocks, num_blocks, retdest
    // STEP 5: write num_blocks to x[0]
    push 0
    %mstore_kernel_general
    // stack: num_blocks, retdest
    %message_schedule_addr_from_num_blocks
    %jump(sha2_gen_all_message_schedules)

global sha2:
    JUMPDEST
    %jump(sha2_store)
