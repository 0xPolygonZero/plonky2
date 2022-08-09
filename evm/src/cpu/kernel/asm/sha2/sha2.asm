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
    //JUMP
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
    // insert 128 (= 1 << 7) at x[num_bytes]
    // stack: num_bytes, retdest
    push 1
    push 7
    swap1 // TODO: remove once SHR implementation is fixed
    shl
    // stack: 128, num_bytes, retdest
    dup2
    // stack: num_bytes, 128, num_bytes, retdest
    %mstore_kernel_general
    // stack: num_bytes, retdest
    // STEP 2: calculate num_blocks := (num_bytes+8)//64 + 1
    dup1
    // stack: num_bytes, num_bytes, retdest
    %add_const(8)
    %div_const(64)
    
    %increment
    // stack: num_blocks = (num_bytes+8)//64 + 1, num_bytes, retdest
    // STEP 3: calculate length := num_bytes*8+1
    swap1
    // stack: num_bytes, num_blocks, retdest
    push 8
    mul
    %increment
    // stack: length = num_bytes*8+1, num_blocks, retdest
    // STEP 4: write length to x[num_blocks*64-8..num_blocks*64-1] 
    dup2
    // stack: num_blocks, length, num_blocks, retdest
    push 64
    mul
    %decrement
    // stack: last_addr = num_blocks*64-1, length, num_blocks, retdest
    %sha2_write_length
    // stack: num_blocks, retdest
    // STEP 5: write num_blocks to x[0]
    push 0
    %mstore_kernel_general
    // stack: retdest
    //JUMP
    push 100
    push 1
    %jump(sha2_gen_message_schedule_from_block)

// Precodition: stack contains address of one message block, followed by output address
// Postcondition: 256 bytes starting at given output address contain the 64 32-bit chunks
//                of message schedule (in four-byte increments)
global sha2_gen_message_schedule_from_block:
    JUMPDEST
    // stack: block_addr, output_addr, retdest
    dup1
    // stack: block_addr, block_addr, output_addr, retdest
    %add_const(32)
    // stack: block_addr + 32, block_addr, output_addr, retdest
    swap1
    // stack: block_addr, block_addr + 32, output_addr, retdest
    %mload_kernel_general_u256
    // stack: block[0], block_addr + 32, output_addr, retdest
    swap1
    // stack: block_addr + 32, block[0], output_addr, retdest
    %mload_kernel_general_u256
    // stack: block[1], block[0], output_addr, retdest
    swap2
    // stack: output_addr, block[0], block[1], retdest
    push 8
    // stack: counter=8, output_addr, block[0], block[1], retdest
    %jump(sha2_gen_message_schedule_from_block_0_loop)
sha2_gen_message_schedule_from_block_0_loop:
    JUMPDEST
    // stack: counter, output_addr, block[0], block[1], retdest
    swap2
    // stack: block[0], output_addr, counter, block[1], retdest
    push 15
    push 1
    //shl
    STOP
    // stack: 1 << 32, block[0], output_addr, counter, block[1], retdest
    dup2
    dup2
    // stack: 1 << 32, block[0], 1 << 32, block[0], output_addr, counter, block[1], retdest
    swap1
    // stack: block[0], 1 << 32, 1 << 32, block[0], output_addr, counter, block[1], retdest
    mod
    // stack: block[0] % (1 << 32), 1 << 32, block[0], output_addr, counter, block[1], retdest
    swap2
    // stack: block[0], 1 << 32, block[0] % (1 << 32), output_addr, counter, block[1], retdest
    div
    // stack: block[0] // (1 << 32), block[0] % (1 << 32), output_addr, counter, block[1], retdest
    swap1
    // stack: block[0] % (1 << 32), block[0] // (1 << 32), output_addr, counter, block[1], retdest
    dup3
    // stack: output_addr, block[0] % (1 << 32), block[0] // (1 << 32), output_addr, counter, block[1], retdest
    %mstore_kernel_general_u32
    // stack: block[0] // (1 << 32), output_addr, counter, block[1], retdest
    swap1
    // stack: output_addr, block[0] // (1 << 32), counter, block[1], retdest
    %add_const(4)
    // stack: output_addr + 4, block[0] // (1 << 32), counter, block[1], retdest
    swap1
    // stack: block[0] // (1 << 32), output_addr + 4, counter, block[1], retdest
    swap2
    // stack: counter, output_addr + 4, block[0] // (1 << 32), block[1], retdest
    %decrement
    dup1
    iszero
    %jumpi(sha2_gen_message_schedule_from_block_0_end)
    %jump(sha2_gen_message_schedule_from_block_0_loop)
sha2_gen_message_schedule_from_block_0_end:
    JUMPDEST
    // stack: old counter=0, output_addr, block[0], block[1], retdest
    pop
    push 8
    // stack: counter=8, output_addr, block[0], block[1], retdest
    swap2
    // stack: block[0], output_addr, counter, block[1], retdest
    swap3
    // stack: block[1], output_addr, counter, block[0], retdest
    swap2
    // stack: counter, output_addr, block[1], block[0], retdest
sha2_gen_message_schedule_from_block_1_loop:
    JUMPDEST
    // stack: counter, output_addr, block[1], block[0], retdest
    swap2
    // stack: block[1], output_addr, counter, block[0], retdest
    push 1
    push 32
    swap1 // TODO: remove once SHL implementation is fixed
    shl
    // stack: 1 << 32, block[1], output_addr, counter, block[0], retdest
    dup2
    dup2
    // stack: 1 << 32, block[1], 1 << 32, block[1], output_addr, counter, block[0], retdest
    swap1
    // stack: block[1], 1 << 32, 1 << 32, block[1], output_addr, counter, block[0], retdest
    mod
    // stack: block[1] % (1 << 32), 1 << 32, block[1], output_addr, counter, block[0], retdest
    swap2
    // stack: block[1], 1 << 32, block[1] % (1 << 32), output_addr, counter, block[0], retdest
    div
    // stack: block[1] // (1 << 32), block[1] % (1 << 32), output_addr, counter, block[0], retdest
    swap1
    // stack: block[1] % (1 << 32), block[1] // (1 << 32), output_addr, counter, block[0], retdest
    dup3
    // stack: output_addr, block[1] % (1 << 32), block[1] // (1 << 32), output_addr, counter, block[0], retdest
    %mstore_kernel_general_u32
    // stack: block[1] // (1 << 32), output_addr, counter, block[0], retdest
    swap1
    // stack: output_addr, block[1] // (1 << 32), counter, block[0], retdest
    %add_const(4)
    // stack: output_addr + 4, block[1] // (1 << 32), counter, block[0], retdest
    swap1
    // stack: block[1] // (1 << 32), output_addr + 4, counter, block[0], retdest
    swap2
    // stack: counter, output_addr + 4, block[1] // (1 << 32), block[0], retdest
    %decrement
    dup1
    iszero
    %jumpi(sha2_gen_message_schedule_from_block_1_end)
    %jump(sha2_gen_message_schedule_from_block_1_loop)
sha2_gen_message_schedule_from_block_1_end:
    JUMPDEST
    // stack: old counter=0, output_addr, block[1], block[0], retdest
    pop
    // stack: output_addr, block[0], block[1], retdest
    push 48
    // stack: counter=48, output_addr, block[0], block[1], retdest
sha2_gen_message_schedule_remaining_loop:
    JUMPDEST
    // stack: counter, output_addr, block[0], block[1], retdest
    swap1
    // stack: output_addr, counter, block[0], block[1], retdest
    dup1
    // stack: output_addr, output_addr, counter, block[0], block[1], retdest
    push 2
    push 4
    mul
    swap1
    sub
    // stack: output_addr - 2*4, output_addr, counter, block[0], block[1], retdest
    %mload_kernel_general_u32
    // stack: x[output_addr - 2*4], output_addr, counter, block[0], block[1], retdest
    %sha2_sigma_1
    // stack: sigma_1(x[output_addr - 2*4]), output_addr, counter, block[0], block[1], retdest
    swap1
    // stack: output_addr, sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    dup1
    // stack: output_addr, output_addr, sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    push 7
    push 4
    mul
    swap1
    sub
    // stack: output_addr - 7*4, output_addr, sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    %mload_kernel_general_u32
    // stack: x[output_addr - 7*4], output_addr, sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    swap1
    // stack: output_addr, x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    dup1
    // stack: output_addr, output_addr, x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    push 15
    push 4
    mul
    swap1
    sub
    // stack: output_addr - 15*4, output_addr, x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    %mload_kernel_general_u32
    // stack: x[output_addr - 15*4], output_addr, x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    %sha2_sigma_0
    // stack: sigma_0(x[output_addr - 15*4]), output_addr, x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    swap1
    // stack: output_addr, sigma_0(x[output_addr - 15*4]), x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    dup1
    // stack: output_addr, output_addr, sigma_0(x[output_addr - 15*4]), x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    push 16
    push 4
    mul
    swap1
    sub
    // stack: output_addr - 16*4, output_addr, sigma_0(x[output_addr - 15*4]), x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    %mload_kernel_general_u32
    // stack: x[output_addr - 16*4], output_addr, sigma_0(x[output_addr - 15*4]), x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    swap1
    // stack: output_addr, x[output_addr - 16*4], sigma_0(x[output_addr - 15*4]), x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    swap4
    // stack: sigma_1(x[output_addr - 2*4]), x[output_addr - 16*4], sigma_0(x[output_addr - 15*4]), x[output_addr - 7*4], output_addr, counter, block[0], block[1], retdest
    add
    add
    add
    // stack: sigma_1(x[output_addr - 2*4]) + x[output_addr - 16*4] + sigma_0(x[output_addr - 15*4]) + x[output_addr - 7*4], output_addr, counter, block[0], block[1], retdest
    swap1
    // stack: output_addr, sigma_1(x[output_addr - 2*4]) + x[output_addr - 16*4] + sigma_0(x[output_addr - 15*4]) + x[output_addr - 7*4], counter, block[0], block[1], retdest
    dup1
    // stack: output_addr, output_addr, sigma_1(x[output_addr - 2*4]) + x[output_addr - 16*4] + sigma_0(x[output_addr - 15*4]) + x[output_addr - 7*4], counter, block[0], block[1], retdest
    swap2
    // stack: sigma_1(x[output_addr - 2*4]) + x[output_addr - 16*4] + sigma_0(x[output_addr - 15*4]) + x[output_addr - 7*4], output_addr, output_addr, counter, block[0], block[1], retdest
    swap1
    // stack: output_addr, sigma_1(x[output_addr - 2*4]) + x[output_addr - 16*4] + sigma_0(x[output_addr - 15*4]) + x[output_addr - 7*4], output_addr, counter, block[0], block[1], retdest
    %mstore_kernel_general_u32
    // stack: output_addr, counter, block[0], block[1], retdest
    %add_const(4)
    // stack: output_addr + 4, counter, block[0], block[1], retdest
    swap1
    // stack: counter, output_addr + 4, block[0], block[1], retdest
    %decrement
    // stack: counter - 1, output_addr + 4, block[0], block[1], retdest
    dup1
    iszero
    %jumpi(sha2_gen_message_schedule_remaining_end)
    %jump(sha2_gen_message_schedule_remaining_loop)
sha2_gen_message_schedule_remaining_end:
    JUMPDEST
    // stack: counter=0, output_addr, block[0], block[1], retdest
    %pop4
    JUMP

// Precodition: memory, starting at 0, contains num_blocks, block0[0], ..., block0[63], block1[0], ..., blocklast[63]
                stack contains output_addr
// Postcondition: 
global sha2_gen_all_message_schedules: 
    JUMPDEST
    push 0
    // stack: 0, output_addr, retdest
    %mload_kernel_general
    // stack: num_blocks, output_addr, retdest
    push 1
    // stack: cur_addr = 1, counter = num_blocks, output_addr, retdest
sha2_gen_all_message_schedules_loop:
    JUMPDEST
    // stack: cur_addr, counter, cur_output_addr, retdest
    push sha2_gen_all_message_schedules_loop_end
    // stack: new_retdest = sha2_gen_all_message_schedules_loop_end, cur_addr, counter, cur_output_addr, retdest
    dup4
    // stack: cur_output_addr, new_retdest, cur_addr, counter, cur_output_addr, retdest
    dup3
    // stack: cur_addr, cur_output_addr, new_retdest, cur_addr, counter, cur_output_addr, retdest
    %jump(sha2_gen_message_schedule_from_block)
sha2_gen_all_message_schedules_loop_end:
    // stack: cur_addr, counter, cur_output_addr, retdest
    %add_const(64)
    // stack: cur_addr + 64, counter, cur_output_addr, retdest
    swap1
    %decrement
    swap1
    // stack: cur_addr + 64, counter - 1, cur_output_addr, retdest
    swap2
    %add_const(256)
    swap2
    // stack: cur_addr + 64, counter - 1, cur_output_addr + 256, retdest
    dup2
    // stack: counter - 1, cur_addr + 64, counter - 1, cur_output_addr + 256, retdest
    iszero
    %jumpi(sha2_gen_all_message_schedules_end)
    %jump(sha2_gen_all_message_schedules_loop)
    JUMPDEST
sha2_gen_all_message_schedules_end:
    JUMPDEST
    // stack: cur_addr + 64, counter - 1, cur_output_addr + 256, retdest
    %pop3
    // stack: retdest
    JUMP

