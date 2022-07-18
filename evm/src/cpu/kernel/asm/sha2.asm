

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
    mload
    // stack: block, output_addr
    push 16
    // stack: counter=16, block, output_addr


global sha2_message_schedule_next_word:
    JUMPDEST
    // stack: address





global sha2_gen_message_schedules:
    JUMPDEST