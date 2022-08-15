global sha2:
    JUMPDEST
    %jump(sha2_store)

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
    // STEP 5: write num_blocks to x[0]
    push 0
    %mstore_kernel_general
    // stack: retdest
    push 100
    %jump(sha2_gen_all_message_schedules)

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
    %add_const(28)
    push 8
    // stack: counter=8, output_addr + 28, block[0], block[1], retdest
    %jump(sha2_gen_message_schedule_from_block_0_loop)
sha2_gen_message_schedule_from_block_0_loop:
    JUMPDEST
    // stack: counter, output_addr, block[0], block[1], retdest
    swap2
    // stack: block[0], output_addr, counter, block[1], retdest
    push 1
    push 32
    shl
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
    // stack: block[0] >> 32, block[0] % (1 << 32), output_addr, counter, block[1], retdest
    swap1
    // stack: block[0] % (1 << 32), block[0] >> 32, output_addr, counter, block[1], retdest
    dup3
    // stack: output_addr, block[0] % (1 << 32), block[0] >> 32, output_addr, counter, block[1], retdest
    %mstore_kernel_general_u32
    // stack: block[0] >> 32, output_addr, counter, block[1], retdest
    swap1
    // stack: output_addr, block[0] >> 32, counter, block[1], retdest
    %sub_const(4)
    // stack: output_addr - 4, block[0] >> 32, counter, block[1], retdest
    swap1
    // stack: block[0] >> 32, output_addr - 4, counter, block[1], retdest
    swap2
    // stack: counter, output_addr - 4, block[0] >> 32, block[1], retdest
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
    swap1
    // stack: output_addr, counter, block[1], block[0], retdest
    %add_const(64)
    // stack: output_addr + 64, counter, block[1], block[0], retdest
    swap1
    // stack: counter, output_addr + 64, block[1], block[0], retdest
sha2_gen_message_schedule_from_block_1_loop:
    JUMPDEST
    // stack: counter, output_addr, block[1], block[0], retdest
    swap2
    // stack: block[1], output_addr, counter, block[0], retdest
    push 1
    push 32
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
    // stack: block[1] >> 32, block[1] % (1 << 32), output_addr, counter, block[0], retdest
    swap1
    // stack: block[1] % (1 << 32), block[1] >> 32, output_addr, counter, block[0], retdest
    dup3
    // stack: output_addr, block[1] % (1 << 32), block[1] >> 32, output_addr, counter, block[0], retdest
    %mstore_kernel_general_u32
    // stack: block[1] >> 32, output_addr, counter, block[0], retdest
    swap1
    // stack: output_addr, block[1] >> 32, counter, block[0], retdest
    %sub_const(4)
    // stack: output_addr - 4, block[1] >> 32, counter, block[0], retdest
    swap1
    // stack: block[1] >> 32, output_addr - 4, counter, block[0], retdest
    swap2
    // stack: counter, output_addr - 4, block[1] >> 32, block[0], retdest
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
    swap1
    // stack: output_addr, counter, block[0], block[1], retdest
    %add_const(36)
    // stack: output_addr + 36, counter, block[0], block[1], retdest
    swap1
    // stack: counter, output_addr + 36, block[0], block[1], retdest
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
    %add_u32
    %add_u32
    %add_u32
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
//              stack contains output_addr
// Postcondition: starting at output_addr, set of 256 bytes per block
//                each contains the 64 32-bit chunks of the message schedule for that block (in four-byte increments)
global sha2_gen_all_message_schedules: 
    JUMPDEST
    push 0
    // stack: 0, output_addr, retdest
    dup2
    // stack: output_addr, 0, output_addr, retdest
    swap1
    // stack: 0, output_addr, output_addr, retdest
    %mload_kernel_general
    // stack: num_blocks, output_addr, output_addr, retdest
    push 1
    // stack: cur_addr = 1, counter = num_blocks, output_addr, output_addr, retdest
sha2_gen_all_message_schedules_loop:
    JUMPDEST
    // stack: cur_addr, counter, cur_output_addr, output_addr, retdest
    push sha2_gen_all_message_schedules_loop_end
    // stack: new_retdest = sha2_gen_all_message_schedules_loop_end, cur_addr, counter, cur_output_addr, output_addr, retdest
    dup4
    // stack: cur_output_addr, new_retdest, cur_addr, counter, cur_output_addr, output_addr, retdest
    dup3
    // stack: cur_addr, cur_output_addr, new_retdest, cur_addr, counter, cur_output_addr, output_addr, retdest
    %jump(sha2_gen_message_schedule_from_block)
sha2_gen_all_message_schedules_loop_end:
    JUMPDEST
    // stack: cur_addr, counter, cur_output_addr, output_addr, retdest
    %add_const(64)
    // stack: cur_addr + 64, counter, cur_output_addr, output_addr, retdest
    swap1
    %decrement
    swap1
    // stack: cur_addr + 64, counter - 1, cur_output_addr, output_addr, retdest
    swap2
    %add_const(256)
    swap2
    // stack: cur_addr + 64, counter - 1, cur_output_addr + 256, output_addr, retdest
    dup2
    // stack: counter - 1, cur_addr + 64, counter - 1, cur_output_addr + 256, output_addr, retdest
    iszero
    %jumpi(sha2_gen_all_message_schedules_end)
    %jump(sha2_gen_all_message_schedules_loop)
    JUMPDEST
sha2_gen_all_message_schedules_end:
    JUMPDEST
    // stack: cur_addr + 64, counter - 1, cur_output_addr + 256, output_addr, retdest
    %pop3
    // stack: output_addr, retdest
    push 0
    // stack: 0, output_addr, retdest
    swap1
    // stack: output_addr, 0, retdest
    %jump(sha2_compression)

// TODO: message schedules for multiple blocks
global sha2_compression:
    JUMPDEST
    // stack: message_schedule_addr, i=0, retdest
    push sha2_constants_h
    %add_const(7)
    %mload_kernel_code_u32
    // stack: h[0], message_schedule_addr, i=0, retdest
    push sha2_constants_h
    %add_const(6)
    %mload_kernel_code_u32
    // stack: g[0], h[0], message_schedule_addr, i=0, retdest
    push sha2_constants_h
    %add_const(5)
    %mload_kernel_code_u32
    // stack: f[0], g[0], h[0], message_schedule_addr, i=0, retdest
    push sha2_constants_h
    %add_const(4)
    %mload_kernel_code_u32
    // stack: e[0], f[0], g[0], h[0], message_schedule_addr, i=0, retdest
    push sha2_constants_h
    %add_const(3)
    %mload_kernel_code_u32
    // stack: d[0], e[0], f[0], g[0], h[0], message_schedule_addr, i=0, retdest
    push sha2_constants_h
    %add_const(2)
    %mload_kernel_code_u32
    // stack: c[0], d[0], e[0], f[0], g[0], h[0], message_schedule_addr, i=0, retdest
    push sha2_constants_h
    %add_const(1)
    %mload_kernel_code_u32
    // stack: b[0], c[0], d[0], e[0], f[0], g[0], h[0], message_schedule_addr, i=0, retdest
    push sha2_constants_h
    %mload_kernel_code_u32
    // stack: a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], message_schedule_addr, i=0, retdest
sha2_compression_loop:
    JUMPDEST
    // stack: a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    dup9
    // stack: message_schedule_addr, a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    dup11
    // stack: i, message_schedule_addr, a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    %mul_const(4)
    // stack: 4*i, message_schedule_addr, a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    add
    // stack: message_schedule_addr + 4*i, a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    %mload_kernel_general_u32
    // stack: W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    push sha2_constants_k
    // stack: sha2_constants_k, W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    dup12
    // stack: i, sha2_constants_k, W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    %mul_const(4)
    // stack: 4*i, sha2_constants_k, W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    add
    // stack: sha2_constants_k + 4*i, W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    %mload_kernel_code_u32
    // stack: K[i], W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    dup10
    // stack: h[i], K[i], W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    dup10
    // stack: g[i], h[i], K[i], W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    dup10
    // stack: f[i], g[i], h[i], K[i], W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    dup10
    // stack: e[i], f[i], g[i], h[i], K[i], W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    %sha2_temp_word1
    // stack: T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    dup4
    // stack: c[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    dup4
    // stack: b[i], c[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    dup4
    // stack: a[i], b[i], c[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    %sha2_temp_word2
    // stack: T2[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    dup6
    // stack: d[i], T2[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    dup3
    // stack: T[i], d[i], T2[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    %add_u32
    // stack: e[i+1]=T[i]+d[i], T2[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    swap2
    // stack: T[1], T2[i], e[i+1], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], message_schedule_addr, i, retdest
    %add_u32
    // stack: a[i+1]=T[1]+T2[i], e[i+1], b[i+1]=a[i], c[i+1]=b[i], d[i+1]=c[i], d[i], f[i+1]=e[i], g[i+1]=f[i], h[i+1]=g[i], h[i], message_schedule_addr, i, retdest
    swap1
    // stack: e[i+1], a[i+1], b[i+1], c[i+1], d[i+1], d[i], f[i+1], g[i+1], h[i+1], h[i], message_schedule_addr, i, retdest
    swap5
    // stack: d[i], a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], h[i], message_schedule_addr, i, retdest
    pop
    // stack: a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], h[i], message_schedule_addr, i, retdest
    swap8
    // stack: h[i], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], a[i+1], message_schedule_addr, i, retdest
    pop
    // stack: b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], a[i+1], message_schedule_addr, i, retdest
    swap7
    // stack: a[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], b[i+1], message_schedule_addr, i, retdest
    swap1
    swap7
    swap1
    // stack: a[i+1], b[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], c[i+1], message_schedule_addr, i, retdest
    swap2
    swap7
    swap2
    // stack: a[i+1], b[i+1], c[i+1], e[i+1], f[i+1], g[i+1], h[i+1], d[i+1], message_schedule_addr, i, retdest
    swap3
    swap7
    swap3
    // stack: a[i+1], b[i+1], c[i+1], d[i+1], f[i+1], g[i+1], h[i+1], e[i+1], message_schedule_addr, i, retdest
    swap4
    swap7
    swap4
    // stack: a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], g[i+1], h[i+1], f[i+1], message_schedule_addr, i, retdest
    swap5
    swap7
    swap5
    // stack: a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], h[i+1], g[i+1], message_schedule_addr, i, retdest
    swap6
    swap7
    swap6
    // stack: a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], message_schedule_addr, i, retdest
    dup10
    // stack: i, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], message_schedule_addr, i, retdest
    %increment
    // stack: i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], message_schedule_addr, i, retdest
    dup1
    // stack: i+1, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], message_schedule_addr, i, retdest
    %eq_const(64)
    %jumpi(sha2_compression_end)
    // stack: i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], message_schedule_addr, i, retdest
    swap10
    // stack: i, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], message_schedule_addr, i+1, retdest
    pop
    // stack: a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], message_schedule_addr, i+1, retdest
    %jump(sha2_compression_loop)
sha2_compression_end:
    JUMPDEST
    // stack: i+1=64, a[64], b[64], c[64], d[64], e[64], f[64], g[64], h[64], message_schedule_addr, i, retdest
    pop
    // stack: a[64], b[64], c[64], d[64], e[64], f[64], g[64], h[64], message_schedule_addr, i, retdest
    push sha2_constants_h
    %mload_kernel_code_u32
    // stack: a[0], a[64], b[64], c[64], d[64], e[64], f[64], g[64], h[64], message_schedule_addr, i, retdest
    %add_u32
    // stack: a[0]+a[64], b[64], c[64], d[64], e[64], f[64], g[64], h[64], message_schedule_addr, i, retdest
    swap1
    // stack: b[64], a[0]+a[64], c[64], d[64], e[64], f[64], g[64], h[64], message_schedule_addr, i, retdest
    push sha2_constants_h
    %add_const(1)
    %mload_kernel_code_u32
    // stack: b[0], b[64], a[0]+a[64], c[64], d[64], e[64], f[64], g[64], h[64], message_schedule_addr, i, retdest
    %add_u32
    // stack: b[0]+b[64], a[0]+a[64], c[64], d[64], e[64], f[64], g[64], h[64], message_schedule_addr, i, retdest
    swap2
    // stack: c[64], a[0]+a[64], b[0]+b[64], d[64], e[64], f[64], g[64], h[64], message_schedule_addr, i, retdest
    push sha2_constants_h
    %add_const(2)
    %mload_kernel_code_u32
    // stack: c[0], c[64], a[0]+a[64], b[0]+b[64], d[64], e[64], f[64], g[64], h[64], message_schedule_addr, i, retdest
    %add_u32
    // stack: c[0]+c[64], a[0]+a[64], b[0]+b[64], d[64], e[64], f[64], g[64], h[64], message_schedule_addr, i, retdest
    swap3
    // stack: d[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], e[64], f[64], g[64], h[64], message_schedule_addr, i, retdest
    push sha2_constants_h
    %add_const(3)
    %mload_kernel_code_u32
    // stack: d[0], d[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], e[64], f[64], g[64], h[64], message_schedule_addr, i, retdest
    %add_u32
    // stack: d[0]+d[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], e[64], f[64], g[64], h[64], message_schedule_addr, i, retdest
    swap4
    // stack: e[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], f[64], g[64], h[64], message_schedule_addr, i, retdest
    push sha2_constants_h
    %add_const(4)
    %mload_kernel_code_u32
    // stack: e[0], e[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], f[64], g[64], h[64], message_schedule_addr, i, retdest
    %add_u32
    // stack: e[0]+e[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], f[64], g[64], h[64], message_schedule_addr, i, retdest
    swap5
    // stack: f[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], g[64], h[64], message_schedule_addr, i, retdest
    push sha2_constants_h
    %add_const(5)
    %mload_kernel_code_u32
    // stack: f[0], f[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], g[64], h[64], message_schedule_addr, i, retdest
    %add_u32
    // stack: f[0]+f[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], g[64], h[64], message_schedule_addr, i, retdest
    swap6
    // stack: g[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], h[64], message_schedule_addr, i, retdest
    push sha2_constants_h
    %add_const(6)
    %mload_kernel_code_u32
    // stack: g[0], g[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], h[64], message_schedule_addr, i, retdest
    %add_u32
    // stack: g[0]+g[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], h[64], message_schedule_addr, i, retdest
    swap7
    // stack: h[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], message_schedule_addr, i, retdest
    push sha2_constants_h
    %add_const(6)
    %mload_kernel_code_u32
    // stack: h[0], h[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], message_schedule_addr, i, retdest
    %add_u32
    // stack: h[0]+h[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], message_schedule_addr, i, retdest
    swap8
    // stack: message_schedule_addr, a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], h[0]+h[64], i, retdest
    pop
    // stack: a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], h[0]+h[64], i, retdest
    swap1
    %shl_const(32)
    or
    swap1
    %shl_const(64)
    or
    swap1
    %shl_const(96)
    or
    swap1
    %shl_const(128)
    or
    swap1
    %shl_const(160)
    or
    swap1
    %shl_const(192)
    or
    swap1
    %shl_const(224)
    or
    // stack: concat(h[0]+h[64], g[0]+g[64], f[0]+f[64], e[0]+e[64], d[0]+d[64], c[0]+c[64], b[0]+b[64], a[0]+a[64]), i, retdest
    swap1
    // stack: i, concat(h[0]+h[64], g[0]+g[64], f[0]+f[64], e[0]+e[64], d[0]+d[64], c[0]+c[64], b[0]+b[64], a[0]+a[64]), retdest
    pop
    // stack: concat(h[0]+h[64], g[0]+g[64], f[0]+f[64], e[0]+e[64], d[0]+d[64], c[0]+c[64], b[0]+b[64], a[0]+a[64]), retdest
    STOP
