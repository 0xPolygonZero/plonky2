// We put the message schedule in memory starting at 64 * num_blocks + 2.
%macro message_schedule_addr_from_num_blocks
    // stack: num_blocks
    %mul_const(64)
    %add_const(2)
%endmacro

// Precodition: stack contains address of one message block, followed by output address
// Postcondition: 256 bytes starting at given output address contain the 64 32-bit chunks
//                of message schedule (in four-byte increments)
gen_message_schedule_from_block:
    // stack: block_addr, output_addr, retdest
    DUP1
    // stack: block_addr, block_addr, output_addr, retdest
    %add_const(32)
    // stack: block_addr + 32, block_addr, output_addr, retdest
    SWAP1
    // stack: block_addr, block_addr + 32, output_addr, retdest
    %mload_kernel_general_u256
    // stack: block[0], block_addr + 32, output_addr, retdest
    SWAP1
    // stack: block_addr + 32, block[0], output_addr, retdest
    %mload_kernel_general_u256
    // stack: block[1], block[0], output_addr, retdest
    SWAP2
    // stack: output_addr, block[0], block[1], retdest
    %add_const(28)
    PUSH 8
    // stack: counter=8, output_addr + 28, block[0], block[1], retdest
    %jump(gen_message_schedule_from_block_0_loop)
gen_message_schedule_from_block_0_loop:
    // Split the first half (256 bits) of the block into the first eight (32-bit) chunks of the message sdchedule.
    // stack: counter, output_addr, block[0], block[1], retdest
    SWAP2
    // stack: block[0], output_addr, counter, block[1], retdest
    DUP1
    // stack: block[0], block[0], output_addr, counter, block[1], retdest
    %shr_const(32)
    // stack: block[0] >> 32, block[0], output_addr, counter, block[1], retdest
    SWAP1
    // stack: block[0], block[0] >> 32, output_addr, counter, block[1], retdest
    %as_u32
    // stack: block[0] % (1 << 32), block[0] >> 32, output_addr, counter, block[1], retdest
    DUP3
    // stack: output_addr, block[0] % (1 << 32), block[0] >> 32, output_addr, counter, block[1], retdest
    %mstore_kernel_general_u32
    // stack: block[0] >> 32, output_addr, counter, block[1], retdest
    SWAP1
    // stack: output_addr, block[0] >> 32, counter, block[1], retdest
    %sub_const(4)
    // stack: output_addr - 4, block[0] >> 32, counter, block[1], retdest
    SWAP1
    // stack: block[0] >> 32, output_addr - 4, counter, block[1], retdest
    SWAP2
    // stack: counter, output_addr - 4, block[0] >> 32, block[1], retdest
    %decrement
    DUP1
    ISZERO
    %jumpi(gen_message_schedule_from_block_0_end)
    %jump(gen_message_schedule_from_block_0_loop)
gen_message_schedule_from_block_0_end:
    // stack: old counter=0, output_addr, block[0], block[1], retdest
    POP
    PUSH 8
    // stack: counter=8, output_addr, block[0], block[1], retdest
    %stack (counter, out, b0, b1) -> (out, counter, b1, b0)
    // stack: output_addr, counter, block[1], block[0], retdest
    %add_const(64)
    // stack: output_addr + 64, counter, block[1], block[0], retdest
    SWAP1
    // stack: counter, output_addr + 64, block[1], block[0], retdest
gen_message_schedule_from_block_1_loop:
    // Split the second half (256 bits) of the block into the next eight (32-bit) chunks of the message sdchedule.
    // stack: counter, output_addr, block[1], block[0], retdest
    SWAP2
    // stack: block[1], output_addr, counter, block[0], retdest
    DUP1
    // stack: block[1], block[1], output_addr, counter, block[0], retdest
    %shr_const(32)
    // stack: block[1] >> 32, block[1], output_addr, counter, block[0], retdest
    SWAP1
    // stack: block[1], block[1] >> 32, output_addr, counter, block[0], retdest
    %as_u32
    // stack: block[1] % (1 << 32), block[1] >> 32, output_addr, counter, block[0], retdest
    DUP3
    // stack: output_addr, block[1] % (1 << 32), block[1] >> 32, output_addr, counter, block[0], retdest
    %mstore_kernel_general_u32
    // stack: block[1] >> 32, output_addr, counter, block[0], retdest
    SWAP1
    // stack: output_addr, block[1] >> 32, counter, block[0], retdest
    %sub_const(4)
    // stack: output_addr - 4, block[1] >> 32, counter, block[0], retdest
    SWAP1
    // stack: block[1] >> 32, output_addr - 4, counter, block[0], retdest
    SWAP2
    // stack: counter, output_addr - 4, block[1] >> 32, block[0], retdest
    %decrement
    DUP1
    ISZERO
    %jumpi(gen_message_schedule_from_block_1_end)
    %jump(gen_message_schedule_from_block_1_loop)
gen_message_schedule_from_block_1_end:
    // stack: old counter=0, output_addr, block[1], block[0], retdest
    POP
    // stack: output_addr, block[0], block[1], retdest
    PUSH 48
    // stack: counter=48, output_addr, block[0], block[1], retdest
    SWAP1
    // stack: output_addr, counter, block[0], block[1], retdest
    %add_const(36)
    // stack: output_addr + 36, counter, block[0], block[1], retdest
    SWAP1
    // stack: counter, output_addr + 36, block[0], block[1], retdest
gen_message_schedule_remaining_loop:
    // Generate the next 48 chunks of the message schedule, one at a time, from prior chunks.
    // stack: counter, output_addr, block[0], block[1], retdest
    SWAP1
    // stack: output_addr, counter, block[0], block[1], retdest
    DUP1
    // stack: output_addr, output_addr, counter, block[0], block[1], retdest
    PUSH 2
    PUSH 4
    MUL
    SWAP1
    SUB
    // stack: output_addr - 2*4, output_addr, counter, block[0], block[1], retdest
    %mload_kernel_general_u32
    // stack: x[output_addr - 2*4], output_addr, counter, block[0], block[1], retdest
    %sha2_sigma_1
    // stack: sigma_1(x[output_addr - 2*4]), output_addr, counter, block[0], block[1], retdest
    SWAP1
    // stack: output_addr, sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    DUP1
    // stack: output_addr, output_addr, sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    PUSH 7
    PUSH 4
    MUL
    SWAP1
    SUB
    // stack: output_addr - 7*4, output_addr, sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    %mload_kernel_general_u32
    // stack: x[output_addr - 7*4], output_addr, sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    SWAP1
    // stack: output_addr, x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    DUP1
    // stack: output_addr, output_addr, x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    PUSH 15
    PUSH 4
    MUL
    SWAP1
    SUB
    // stack: output_addr - 15*4, output_addr, x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    %mload_kernel_general_u32
    // stack: x[output_addr - 15*4], output_addr, x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    %sha2_sigma_0
    // stack: sigma_0(x[output_addr - 15*4]), output_addr, x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    SWAP1
    // stack: output_addr, sigma_0(x[output_addr - 15*4]), x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    DUP1
    // stack: output_addr, output_addr, sigma_0(x[output_addr - 15*4]), x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    PUSH 16
    PUSH 4
    MUL
    SWAP1
    SUB
    // stack: output_addr - 16*4, output_addr, sigma_0(x[output_addr - 15*4]), x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    %mload_kernel_general_u32
    // stack: x[output_addr - 16*4], output_addr, sigma_0(x[output_addr - 15*4]), x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    SWAP1
    // stack: output_addr, x[output_addr - 16*4], sigma_0(x[output_addr - 15*4]), x[output_addr - 7*4], sigma_1(x[output_addr - 2*4]), counter, block[0], block[1], retdest
    SWAP4
    // stack: sigma_1(x[output_addr - 2*4]), x[output_addr - 16*4], sigma_0(x[output_addr - 15*4]), x[output_addr - 7*4], output_addr, counter, block[0], block[1], retdest
    %add_u32
    %add_u32
    %add_u32
    // stack: sigma_1(x[output_addr - 2*4]) + x[output_addr - 16*4] + sigma_0(x[output_addr - 15*4]) + x[output_addr - 7*4], output_addr, counter, block[0], block[1], retdest
    DUP2
    // stack: output_addr, sigma_1(x[output_addr - 2*4]) + x[output_addr - 16*4] + sigma_0(x[output_addr - 15*4]) + x[output_addr - 7*4], output_addr, counter, block[0], block[1], retdest
    %mstore_kernel_general_u32
    // stack: output_addr, counter, block[0], block[1], retdest
    %add_const(4)
    // stack: output_addr + 4, counter, block[0], block[1], retdest
    SWAP1
    // stack: counter, output_addr + 4, block[0], block[1], retdest
    %decrement
    // stack: counter - 1, output_addr + 4, block[0], block[1], retdest
    DUP1
    ISZERO
    %jumpi(gen_message_schedule_remaining_end)
    %jump(gen_message_schedule_remaining_loop)
gen_message_schedule_remaining_end:
    // stack: counter=0, output_addr, block[0], block[1], retdest
    %pop4
    JUMP

// Precodition: memory, starting at 0, contains num_blocks, block0[0], ..., block0[63], block1[0], ..., blocklast[63]
//              stack contains output_addr
// Postcondition: starting at output_addr, set of 256 bytes per block
//                each contains the 64 32-bit chunks of the message schedule for that block (in four-byte increments)
global sha2_gen_all_message_schedules: 
    // stack: output_addr, retdest
    DUP1
    // stack: output_addr, output_addr, retdest
    PUSH 0
    // stack: 0, output_addr, output_addr, retdest
    %mload_kernel_general
    // stack: num_blocks, output_addr, output_addr, retdest
    PUSH 1
    // stack: cur_addr = 1, counter = num_blocks, output_addr, output_addr, retdest
gen_all_message_schedules_loop:
    // stack: cur_addr, counter, cur_output_addr, output_addr, retdest
    PUSH gen_all_message_schedules_loop_end
    // stack: new_retdest = gen_all_message_schedules_loop_end, cur_addr, counter, cur_output_addr, output_addr, retdest
    DUP4
    // stack: cur_output_addr, new_retdest, cur_addr, counter, cur_output_addr, output_addr, retdest
    DUP3
    // stack: cur_addr, cur_output_addr, new_retdest, cur_addr, counter, cur_output_addr, output_addr, retdest
    %jump(gen_message_schedule_from_block)
gen_all_message_schedules_loop_end:
    // stack: cur_addr, counter, cur_output_addr, output_addr, retdest
    %add_const(64)
    // stack: cur_addr + 64, counter, cur_output_addr, output_addr, retdest
    SWAP1
    %decrement
    SWAP1
    // stack: cur_addr + 64, counter - 1, cur_output_addr, output_addr, retdest
    SWAP2
    %add_const(256)
    SWAP2
    // stack: cur_addr + 64, counter - 1, cur_output_addr + 256, output_addr, retdest
    DUP2
    // stack: counter - 1, cur_addr + 64, counter - 1, cur_output_addr + 256, output_addr, retdest
    ISZERO
    %jumpi(gen_all_message_schedules_end)
    %jump(gen_all_message_schedules_loop)
gen_all_message_schedules_end:
    // stack: cur_addr + 64, counter - 1, cur_output_addr + 256, output_addr, retdest
    %pop3
    // stack: output_addr, retdest
    %jump(sha2_compression)
