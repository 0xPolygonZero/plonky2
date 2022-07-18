

// Precodition: input is in memory, starting at [TODO: fix] 0, of the form
//              num_bytes, x[0], x[1], ..., x[(num_bytes+31)/32-1]
// Postcodition: output is in memory, starting at [TODO: fix] 0, of the form
//               num_blocks, block0[0], block0[1], block1[0], ..., blocklast[1]
global sha2_pad:
    // TODO: use kernel memory, and start address not at 0
    push 0
    mload
    // stack: num_bytes
    // STEP 1: append 1
    // add 1 << (8*(32-k)-1) to x[num_bytes//32], where k := num_bytes%32
    dup1
    // stack: num_bytes, num_bytes
    dup1
    // stack: num_bytes, num_bytes, num_bytes
    push 32
    // stack: 32, num_bytes, num_bytes, num_bytes
    swap1
    // stack: num_bytes, 32, num_bytes, num_bytes
    mod
    // stack: k := num_bytes % 32, num_bytes, num_bytes
    push 32
    sub
    // stack: 32 - k, num_bytes, num_bytes
    push 8
    mul
    // stack: 8 * (32 - k), num_bytes, num_bytes
    %decrement
    // stack: 8 * (32 - k) - 1, num_bytes, num_bytes
    push 1
    swap1
    shl
    // stack: 1 << (8 * (32 - k) - 1), num_bytes, num_bytes
    swap1
    // stack: num_bytes, 1 << (8 * (32 - k) - 1), num_bytes
    push 32
    swap1
    div
    // stack: num_bytes // 32, 1 << (8 * (32 - k) - 1), num_bytes
    dup1
    // stack: num_bytes // 32, num_bytes // 32, 1 << (8 * (32 - k) - 1), num_bytes
    mload
    // stack: x[num_bytes // 32], num_bytes // 32, 1 << (8 * (32 - k) - 1), num_bytes
    swap1
    // stack: num_bytes // 32, x[num_bytes // 32], 1 << (8 * (32 - k) - 1), num_bytes
    swap2
    // stack: x[num_bytes // 32], 1 << (8 * (32 - k) - 1), num_bytes // 32, num_bytes
    add
    // stack: x[num_bytes // 32] + 1 << (8 * (32 - k) - 1), num_bytes // 32, num_bytes
    swap1
    // stack: num_bytes // 32, x[num_bytes // 32] + 1 << (8 * (32 - k) - 1), num_bytes
    mstore
    // stack: num_bytes
    // STEP 2: insert length
    // (add length := num_bytes*8+1 to x[(num_bytes//64)*2-1])
    dup1
    dup1
    // stack: num_bytes, num_bytes, num_bytes
    push 8
    mul
    %increment
    // stack: length := num_bytes*8+1, num_bytes, num_bytes
    swap1
    // stack: num_bytes, length := num_bytes*8+1, num_bytes
    push 64
    swap1
    div
    // stack: num_bytes // 64, length := num_bytes*8+1, num_bytes
    push 2
    mul
    %decrement
    // stack: (num_bytes // 64) * 2 - 1, length := num_bytes*8+1, num_bytes
    dup1
    // stack: (num_bytes // 64) * 2 - 1, (num_bytes // 64) * 2 - 1, length, num_bytes
    mload
    // stack: x[(num_bytes // 64) * 2 - 1], (num_bytes // 64) * 2 - 1, length, num_bytes
    swap1
    // stack: (num_bytes // 64) * 2 - 1, x[(num_bytes // 64) * 2 - 1], length, num_bytes
    swap2
    // stack: length, x[(num_bytes // 64) * 2 - 1], (num_bytes // 64) * 2 - 1, num_bytes
    add
    // stack: x[(num_bytes // 64) * 2 - 1] + length, (num_bytes // 64) * 2 - 1, num_bytes
    swap1
    // stack: (num_bytes // 64) * 2 - 1, x[(num_bytes // 64) * 2 - 1] + length, num_bytes
    mstore
    // stack: num_bytes
    // STEP 3: insert num_blocks at start
    push 64
    swap
    div
    %increment
    // stack: num_blocks := num_bytes // 64 + 1
    push 0
    mstore

// Precodition: stack contains address of one message block, followed by output address
// Postcondition: 64 addresses starting at given output address contain 32-bit chunks of message schedule
global sha2_gen_message_schedule_from_block:
    JUMPDEST
    // stack: block_addr, output_addr
    dup1
    // stack: block_addr, block_addr, output_addr
    %increment
    // stack: block_addr + 1, block_addr, output_addr
    swap1
    // stack: block_addr, block_addr + 1, output_addr
    mload
    // stack: block[0], block_addr + 1, output_addr
    swap1
    // stack: block_addr + 1, block[0], output_addr
    mload
    // stack: block[1], block[0], output_addr
    swap2
    // stack: output_addr, block[0], block[1]
    // stack: output_addr, block[0], block[1]
    push 16
    // stack: counter=16, output_addr, block[0], block[1]
    %jump(sha2_gen_message_schedule_from_block_0_loop)
sha2_gen_message_schedule_from_block_0_loop:
    JUMPDEST
    // stack: counter, output_addr, block[0], block[1]
    swap2
    // stack: block[0], output_addr, counter, block[1]
    push 1
    push 32
    shl
    // stack: 1 << 32, block[0], output_addr, counter, block[1]
    dup2
    dup2
    // stack: 1 << 32, block[0], 1 << 32, block[0], output_addr, counter, block[1]
    swap1
    // stack: block[0], 1 << 32, 1 << 32, block[0], output_addr, counter, block[1]
    mod
    // stack: block[0] % (1 << 32), 1 << 32, block[0], output_addr, counter, block[1]
    swap2
    // stack: block[0], 1 << 32, block[0] % (1 << 32), output_addr, counter, block[1]
    // stack: block[0], 1 << 32, block[0] % (1 << 32), output_addr, counter, block[1]
    div
    // stack: block[0] // (1 << 32), block[0] % (1 << 32), output_addr, counter, block[1]
    swap1
    // stack: block[0] % (1 << 32), block[0] // (1 << 32), output_addr, counter, block[1]
    dup3
    // stack: output_addr, block[0] % (1 << 32), block[0] // (1 << 32), output_addr, counter, block[1]
    mstore
    // stack: block[0] // (1 << 32), output_addr, counter, block[1]
    swap1
    // stack: output_addr, block[0] // (1 << 32), counter, block[1]
    %increment
    // stack: output_addr + 1, block[0] // (1 << 32), counter, block[1]
    swap1
    // stack: block[0] // (1 << 32), output_addr + 1, counter, block[1]
    swap2
    // stack: counter, output_addr + 1, block[0] // (1 << 32), block[1]
    %decrement
    dup1
    iszero
    %jumpi(sha2_gen_message_schedule_from_block_0_end)
    %jump(sha2_gen_message_schedule_from_block_0_loop)
sha2_gen_message_schedule_from_block_0_end:
    JUMPDEST
    // stack: old counter=0, 
    pop
    push 16



global sha2_message_schedule_next_word:
    JUMPDEST
    // stack: addr
    dup1
    // stack: addr, addr
    push 2
    swap1
    sub
    // stack: addr - 2, addr
    mload
    // stack: x[addr - 2], addr
    %jump(sha2_sigma_1)
    // stack: sigma_1(x[addr - 2]), addr
    swap1
    // stack: addr, sigma_1(x[addr - 2])
    dup1
    // stack: addr, addr, sigma_1(x[addr - 2])
    push 7
    swap1
    sub
    // stack: addr - 7, addr, sigma_1(x[addr - 2])
    mload
    // stack: x[addr - 7], addr, sigma_1(x[addr - 2])
    swap1
    // stack: addr, x[addr - 7], sigma_1(x[addr - 2])
    dup1
    // stack: addr, addr, x[addr - 7], sigma_1(x[addr - 2])
    push 15
    swap1
    sub
    // stack: addr - 15, addr, x[addr - 7], sigma_1(x[addr - 2])
    mload
    // stack: x[addr - 15], addr, x[addr - 7], sigma_1(x[addr - 2])
    %jump(sha2_sigma_0)
    // stack: sigma_0(x[addr - 15]), addr, x[addr - 7], sigma_1(x[addr - 2])
    swap1
    // stack: addr, sigma_0(x[addr - 15]), x[addr - 7], sigma_1(x[addr - 2])
    dup1
    // stack: addr, addr, sigma_0(x[addr - 15]), x[addr - 7], sigma_1(x[addr - 2])
    push 16
    swap1
    sub
    // stack: addr - 16, addr, sigma_0(x[addr - 15]), x[addr - 7], sigma_1(x[addr - 2])
    mload
    // stack: x[addr - 16], addr, sigma_0(x[addr - 15]), x[addr - 7], sigma_1(x[addr - 2])
    swap1
    // stack: addr, x[addr - 16], sigma_0(x[addr - 15]), x[addr - 7], sigma_1(x[addr - 2])
    swap4
    // stack: sigma_1(x[addr - 2]), x[addr - 16], sigma_0(x[addr - 15]), x[addr - 7], addr
    add
    add
    add
    // stack: sigma_1(x[addr - 2]) + x[addr - 16] + sigma_0(x[addr - 15]) + x[addr - 7], addr
    swap1
    mstore

global sha2_gen_all_message_schedules:
    JUMPDEST
