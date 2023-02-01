global sha2:
    %jump(sha2_store)

global sha2_store:
    // stack: num_bytes, x[0], x[1], ..., x[num_bytes - 1], retdest
    DUP1
    // stack: num_bytes, num_bytes, x[0], x[1], ..., x[num_bytes - 1], retdest
    PUSH 0
    // stack: addr=0, num_bytes, num_bytes, x[0], x[1], ..., x[num_bytes - 1], retdest
    %mstore_kernel_general
    // stack: num_bytes, x[0], x[1], ..., x[num_bytes - 1], retdest
    PUSH 1
    // stack: addr=1, counter=num_bytes, x[0], x[1], x[2], ... , x[num_bytes-1], retdest
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
    %jump(sha2_pad)

// Precodition: input is in memory, starting at 0 of kernel general segment, of the form
//              num_bytes, x[0], x[1], ..., x[num_bytes - 1]
// Postcodition: output is in memory, starting at 0, of the form
//               num_blocks, block0[0], ..., block0[63], block1[0], ..., blocklast[63]
global sha2_pad:
    // stack: retdest
    PUSH 0
    %mload_kernel_general
    // stack: num_bytes, retdest
    // STEP 1: append 1
    // insert 128 (= 1 << 7) at x[num_bytes+1]
    // stack: num_bytes, retdest
    PUSH 1
    PUSH 7
    SHL
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
    PUSH 8
    MUL
    // stack: length = num_bytes*8, num_blocks, retdest
    // STEP 4: write length to x[num_blocks*64-7..num_blocks*64]
    DUP2
    // stack: num_blocks, length, num_blocks, retdest
    PUSH 64
    MUL
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
