// We use memory starting at 320 * num_blocks + 2 (after the message schedule
// space) as scratch space to store stack values.
%macro scratch_space_addr_from_num_blocks
    // stack: num_blocks
    %mul_const(320)
    %add_const(2)
%endmacro

global sha2_compression:
    // stack: message_schedule_addr, retdest
    // Push the initial hash values; these constants are called H^(0) in the spec.
    PUSH 0x1f83d9ab // H^(0)_6
    PUSH 0x9b05688c // H^(0)_5
    PUSH 0x510e527f // H^(0)_4
    PUSH 0xa54ff53a // H^(0)_3
    PUSH 0x3c6ef372 // H^(0)_2
    PUSH 0xbb67ae85 // H^(0)_1
    PUSH 0x6a09e667 // H^(0)_0
    PUSH 0x5be0cd19 // H^(0)_7
    // stack: h[0], a[0], b[0], c[0], d[0], e[0], f[0], g[0], message_schedule_addr, retdest
    SWAP8
    // stack: message_schedule_addr, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], retdest
    PUSH 0
    // stack: i=0, message_schedule_addr, a[0]..h[0], retdest
    SWAP1
    // stack: message_schedule_addr, i=0, a[0]..h[0], retdest
    PUSH 0
    // stack: 0, message_schedule_addr, i=0, a[0]..h[0], retdest
    %mload_current_general
    // stack: num_blocks, message_schedule_addr, i=0, a[0]..h[0], retdest
    DUP1
    // stack: num_blocks, num_blocks, message_schedule_addr, i=0, a[0]..h[0], retdest
    %scratch_space_addr_from_num_blocks
    // stack: scratch_space_addr, num_blocks, message_schedule_addr, i=0, a[0]..h[0], retdest
    SWAP1
    // stack: num_blocks, scratch_space_addr, message_schedule_addr, i=0, a[0]..h[0], retdest
compression_start_block:
    // We keep the current values of the working variables saved at the end of the stack.
    // These are the "initial values" to be added back in at the end of this block.
    // stack: num_blocks, scratch_space_addr, message_schedule_addr, i=0, a[0]..h[0], retdest
    %rep 8
        DUP12
    %endrep
    // stack: a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, a[0]..h[0], retdest
compression_loop:
    // Update the eight working variables, using the next constant K[i] and the next message schedule chunk W[i].
    // stack: a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    DUP11
    // stack: message_schedule_addr, a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    DUP13
    // stack: i, message_schedule_addr, a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    %mul_const(4)
    // stack: 4*i, message_schedule_addr, a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    ADD
    // stack: message_schedule_addr + 4*i, a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    %mload_current_general_u32
    // stack: W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    PUSH sha2_constants_k
    // stack: sha2_constants_k, W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    DUP14
    // stack: i, sha2_constants_k, W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    %mul_const(4)
    // stack: 4*i, sha2_constants_k, W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    ADD
    // stack: sha2_constants_k + 4*i, W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    %mload_kernel_code_u32
    // stack: K[i], W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    DUP10
    DUP10
    DUP10
    DUP10
    // stack: e[i], f[i], g[i], h[i], K[i], W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    %sha2_temp_word1
    // stack: T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    DUP4
    DUP4
    DUP4
    // stack: a[i], b[i], c[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    %sha2_temp_word2
    // stack: T2[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    DUP6
    // stack: d[i], T2[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    DUP3
    // stack: T1[i], d[i], T2[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    %add_u32
    // stack: e[i+1]=T1[i]+d[i], T2[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    SWAP2
    // stack: T2[i], T1[i], e[i+1], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    %add_u32
    // stack: a[i+1]=T1[i]+T2[i], e[i+1], b[i+1]=a[i], c[i+1]=b[i], d[i+1]=c[i], d[i], f[i+1]=e[i], g[i+1]=f[i], h[i+1]=g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    %stack (a, e, b, c, d, old_d, f, g, h, old_h) -> (a, b, c, d, e, f, g, h)
    // stack: a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    DUP12
    // stack: i, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    %increment
    // stack: i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    DUP1
    // stack: i+1, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    %eq_const(64)
    // stack: i+1==64, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    DUP1
    // stack: i+1==64, i+1==64, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    DUP12
    // stack: num_blocks, i+1==64, i+1==64, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    SUB
    // stack: num_blocks new, i+1==64, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]..h[0], retdest
    SWAP13
    // stack: message_schedule_addr, i+1==64, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, num_blocks new, i, a[0]..h[0], retdest
    SWAP1
    // stack: i+1==64, message_schedule_addr, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, num_blocks new, i, a[0]..h[0], retdest
    %mul_const(256)
    // stack: (i+1==64)*256, message_schedule_addr, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, num_blocks new, i, a[0]..h[0], retdest
    ADD
    // stack: message_schedule_addr new, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, num_blocks new, i, a[0]..h[0], retdest
    SWAP12
    // stack: num_blocks new, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, message_schedule_addr new, i, a[0]..h[0], retdest
    SWAP10
    // stack: num_blocks, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks new, scratch_space_addr, message_schedule_addr new, i, new_a[0]..h[0], retdest
    POP
    // stack: i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks new, scratch_space_addr, message_schedule_addr new, i, new_a[0]..h[0], retdest
    %and_const(63)
    // stack: (i+1)%64, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks new, scratch_space_addr, message_schedule_addr new, i, a[0]..h[0], retdest
    SWAP12
    // stack: i, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks new, scratch_space_addr, message_schedule_addr new, (i+1)%64, a[0]..h[0], retdest
    POP
    // stack: a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks new, scratch_space_addr, message_schedule_addr new, (i+1)%64, a[0]..h[0], retdest
    DUP12
    // stack: (i+1)%64, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks new, scratch_space_addr, message_schedule_addr new, (i+1)%64, a[0]..h[0], retdest
    %jumpi(compression_loop)
compression_end_block:
    // Add the initial values of the eight working variables (from the start of this block's compression) back into them.
    // stack: a[64], b[64], c[64], d[64], e[64], f[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], retdest
    PUSH 0
    // stack: 0, a[64], b[64], c[64], d[64], e[64], f[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], retdest
    %rep 8
        SWAP13
        %add_u32
        SWAP12
    %endrep
    // stack: 0, num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], h[0]+h[64], retdest
    POP
    // stack: num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], h[0]+h[64], retdest
    DUP1
    // stack: num_blocks, num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], h[0]+h[64], retdest
    ISZERO
    // In this case, we've finished all the blocks.
    %jumpi(compression_end)
    // stack: num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], h[0]+h[64], retdest
    %jump(compression_start_block)
compression_end:
    // stack: num_blocks, scratch_space_addr, message_schedule_addr, i, a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], h[0]+h[64], retdest
    %pop4
    // stack: a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], h[0]+h[64], retdest
    %rep 7
        %shl_const(32)
        ADD // OR
    %endrep
    // stack: sha2_result = concat(a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], h[0]+h[64]), retdest
    SWAP1
    JUMP
