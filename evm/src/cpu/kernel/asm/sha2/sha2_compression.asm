global sha2_compression:
    JUMPDEST
    // stack: message_schedule_addr, retdest
    push 0
    // stack: i=0, message_schedule_addr, retdest
    swap1
    // stack: message_schedule_addr, i=0, retdest
    push 0
    // stack: 0, message_schedule_addr, i=0, retdest
    %mload_kernel_general
    // stack: num_blocks, message_schedule_addr, i=0, retdest
    dup1
    // stack: num_blocks, num_blocks, message_schedule_addr, i=0, retdest
    %scratch_space_addr_from_num_blocks
    // stack: scratch_space_addr, num_blocks, message_schedule_addr, i=0, retdest
    swap1
    // stack: num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    push sha2_constants_h
    %add_const(28)
    %mload_kernel_code_u32
    // stack: h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    push sha2_constants_h
    %add_const(24)
    %mload_kernel_code_u32
    // stack: g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    push sha2_constants_h
    %add_const(20)
    %mload_kernel_code_u32
    // stack: f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    push sha2_constants_h
    %add_const(16)
    %mload_kernel_code_u32
    // stack: e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    push sha2_constants_h
    %add_const(12)
    %mload_kernel_code_u32
    // stack: d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    push sha2_constants_h
    %add_const(8)
    %mload_kernel_code_u32
    // stack: c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    push sha2_constants_h
    %add_const(4)
    %mload_kernel_code_u32
    // stack: b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    push sha2_constants_h
    %mload_kernel_code_u32
    // stack: a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    dup10
    // stack: scratch_space_addr, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest

    dup2
    dup2
    // stack: scratch_space_addr, a[0], scratch_space_addr, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    %mstore_kernel_general_u32
    // stack: scratch_space_addr, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    %add_const(4)
    // stack: scratch_space_addr+4, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest

    dup3
    dup2
    // stack: scratch_space_addr+4, b[0], scratch_space_addr+4, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    %mstore_kernel_general_u32
    // stack: scratch_space_addr+4, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    %add_const(4)
    // stack: scratch_space_addr+8, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    
    dup4
    dup2
    // stack: scratch_space_addr+8, c[0], scratch_space_addr+8, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    %mstore_kernel_general_u32
    // stack: scratch_space_addr+8, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    %add_const(4)
    // stack: scratch_space_addr+12, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    
    dup5
    dup2
    // stack: scratch_space_addr+12, c[0], scratch_space_addr+8, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    %mstore_kernel_general_u32
    // stack: scratch_space_addr+12, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    %add_const(4)
    // stack: scratch_space_addr+16, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    
    dup6
    dup2
    // stack: scratch_space_addr+16, c[0], scratch_space_addr+8, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    %mstore_kernel_general_u32
    // stack: scratch_space_addr+16, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    %add_const(4)
    // stack: scratch_space_addr+20, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    
    dup7
    dup2
    // stack: scratch_space_addr+20, c[0], scratch_space_addr+8, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    %mstore_kernel_general_u32
    // stack: scratch_space_addr+20, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    %add_const(4)
    // stack: scratch_space_addr+24, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    
    dup8
    dup2
    // stack: scratch_space_addr+24, c[0], scratch_space_addr+8, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    %mstore_kernel_general_u32
    // stack: scratch_space_addr+24, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    %add_const(4)
    // stack: scratch_space_addr+28, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest

    dup9
    dup2
    // stack: scratch_space_addr+28, c[0], scratch_space_addr+8, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    %mstore_kernel_general_u32
    // stack: scratch_space_addr+28, a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
    pop
    // stack: a[0], b[0], c[0], d[0], e[0], f[0], g[0], h[0], num_blocks, scratch_space_addr, message_schedule_addr, i=0, retdest
sha2_compression_loop:
    JUMPDEST
    // stack: a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup11
    // stack: message_schedule_addr, a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup13
    // stack: i, message_schedule_addr, a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %mul_const(4)
    // stack: 4*i, message_schedule_addr, a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    add
    // stack: message_schedule_addr + 4*i, a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %mload_kernel_general_u32
    // stack: W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    push sha2_constants_k
    // stack: sha2_constants_k, W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup14
    // stack: i, sha2_constants_k, W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %mul_const(4)
    // stack: 4*i, sha2_constants_k, W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    add
    // stack: sha2_constants_k + 4*i, W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %mload_kernel_code_u32
    // stack: K[i], W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup10
    // stack: h[i], K[i], W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup10
    // stack: g[i], h[i], K[i], W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup10
    // stack: f[i], g[i], h[i], K[i], W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup10
    // stack: e[i], f[i], g[i], h[i], K[i], W[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %sha2_temp_word1
    // stack: T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup4
    // stack: c[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup4
    // stack: b[i], c[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup4
    // stack: a[i], b[i], c[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %sha2_temp_word2
    // stack: T2[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup6
    // stack: d[i], T2[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup3
    // stack: T1[i], d[i], T2[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %add_u32
    // stack: e[i+1]=T1[i]+d[i], T2[i], T1[i], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap2
    // stack: T2[i], T1[i], e[i+1], a[i], b[i], c[i], d[i], e[i], f[i], g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %add_u32
    // stack: a[i+1]=T1[i]+T2[i], e[i+1], b[i+1]=a[i], c[i+1]=b[i], d[i+1]=c[i], d[i], f[i+1]=e[i], g[i+1]=f[i], h[i+1]=g[i], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap1
    // stack: e[i+1], a[i+1], b[i+1], c[i+1], d[i+1], d[i], f[i+1], g[i+1], h[i+1], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap5
    // stack: d[i], a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    pop
    // stack: a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], h[i], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap8
    // stack: h[i], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], a[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    pop
    // stack: b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], a[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap7
    // stack: a[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], b[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap1
    swap7
    swap1
    // stack: a[i+1], b[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], c[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap2
    swap7
    swap2
    // stack: a[i+1], b[i+1], c[i+1], e[i+1], f[i+1], g[i+1], h[i+1], d[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap3
    swap7
    swap3
    // stack: a[i+1], b[i+1], c[i+1], d[i+1], f[i+1], g[i+1], h[i+1], e[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap4
    swap7
    swap4
    // stack: a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], g[i+1], h[i+1], f[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap5
    swap7
    swap5
    // stack: a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], h[i+1], g[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap6
    swap7
    swap6
    // stack: a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup12
    // stack: i, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %increment
    // stack: i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup1
    // stack: i+1, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %eq_const(64)
    // stack: i+1==64, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup1
    // stack: i+1==64, i+1==64, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup12
    // stack: num_blocks, i+1==64, i+1==64, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    sub
    // stack: num_blocks new, i+1==64, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap13
    // stack: message_schedule_addr, i+1==64, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, num_blocks new, i, retdest
    swap1
    // stack: i+1==64, message_schedule_addr, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, num_blocks new, i, retdest
    push 256
    mul
    // stack: (i+1==64)*256, message_schedule_addr, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, num_blocks new, i, retdest
    add
    // stack: message_schedule_addr new, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, num_blocks new, i, retdest
    swap12
    // stack: num_blocks new, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks, scratch_space_addr, message_schedule_addr new, i, retdest
    swap10
    // stack: num_blocks, i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks new, scratch_space_addr, message_schedule_addr new, i, new_retdest
    pop
    // stack: i+1, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks new, scratch_space_addr, message_schedule_addr new, i, new_retdest
    push 64
    swap1
    mod
    // stack: (i+1)%64, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks new, scratch_space_addr, message_schedule_addr new, i, retdest
    swap12
    // stack: i, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks new, scratch_space_addr, message_schedule_addr new, (i+1)%64, retdest
    pop
    // stack: a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks new, scratch_space_addr, message_schedule_addr new, (i+1)%64, retdest
    dup12
    // stack: (i+1)%64, a[i+1], b[i+1], c[i+1], d[i+1], e[i+1], f[i+1], g[i+1], h[i+1], num_blocks new, scratch_space_addr, message_schedule_addr new, (i+1)%64, retdest
    //dup10
    //iszero
    //dup2
    //iszero
    //and
    //%jumpi(sha2_stop_lol)
    iszero
    %jumpi(sha2_compression_end_block)
    %jump(sha2_compression_loop)
sha2_compression_end_block:
    JUMPDEST
    // stack: a[64], b[64], c[64], d[64], e[64], f[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup10
    // stack: scratch_space_addr, a[64], b[64], c[64], d[64], e[64], f[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %mload_kernel_general_u32
    // stack: a[0], a[64], b[64], c[64], d[64], e[64], f[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %add_u32
    // stack: a[0]+a[64], b[64], c[64], d[64], e[64], f[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap1
    // stack: b[64], a[0]+a[64], c[64], d[64], e[64], f[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup10
    %add_const(4)
    %mload_kernel_general_u32
    // stack: b[0], b[64], a[0]+a[64], c[64], d[64], e[64], f[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %add_u32
    // stack: b[0]+b[64], a[0]+a[64], c[64], d[64], e[64], f[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap2
    // stack: c[64], a[0]+a[64], b[0]+b[64], d[64], e[64], f[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup10
    %add_const(8)
    %mload_kernel_general_u32
    // stack: c[0], c[64], a[0]+a[64], b[0]+b[64], d[64], e[64], f[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %add_u32
    // stack: c[0]+c[64], a[0]+a[64], b[0]+b[64], d[64], e[64], f[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap3
    // stack: d[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], e[64], f[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup10
    %add_const(12)
    %mload_kernel_general_u32
    // stack: d[0], d[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], e[64], f[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %add_u32
    // stack: d[0]+d[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], e[64], f[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap4
    // stack: e[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], f[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup10
    %add_const(16)
    %mload_kernel_general_u32
    // stack: e[0], e[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], f[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %add_u32
    // stack: e[0]+e[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], f[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap5
    // stack: f[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup10
    %add_const(20)
    %mload_kernel_general_u32
    // stack: f[0], f[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %add_u32
    // stack: f[0]+f[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], g[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap6
    // stack: g[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup10
    %add_const(24)
    %mload_kernel_general_u32
    // stack: g[0], g[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %add_u32
    // stack: g[0]+g[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap7
    // stack: h[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup10
    %add_const(28)
    %mload_kernel_general_u32
    // stack: h[0], h[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    %add_u32
    // stack: h[0]+h[64], a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    swap8
    // stack: num_blocks, a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], h[0]+h[64], scratch_space_addr, message_schedule_addr, i, retdest
    dup1
    // stack: num_blocks, num_blocks, a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], h[0]+h[64], scratch_space_addr, message_schedule_addr, i, retdest
    iszero
    %jumpi(sha2_compression_end)
    // stack: num_blocks, a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], h[0]+h[64], scratch_space_addr, message_schedule_addr, i, retdest
    // TODO: "insertion" macro for the below
    swap1
    swap2
    swap1
    swap2
    swap3
    swap2
    swap3
    swap4
    swap3
    swap4
    swap5
    swap4
    swap5
    swap6
    swap5
    swap6
    swap7
    swap6
    swap7
    swap8
    swap7
    swap8
    // stack: a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], h[0]+h[64], num_blocks, scratch_space_addr, message_schedule_addr, i, retdest
    dup1
    dup11
    %mstore_kernel_general_u32
    dup2
    dup11
    %add_const(4)
    %mstore_kernel_general_u32
    dup3
    dup11
    %add_const(8)
    %mstore_kernel_general_u32
    dup4
    dup11
    %add_const(12)
    %mstore_kernel_general_u32
    dup5
    dup11
    %add_const(16)
    %mstore_kernel_general_u32
    dup6
    dup11
    %add_const(20)
    %mstore_kernel_general_u32
    dup7
    dup11
    %add_const(24)
    %mstore_kernel_general_u32
    dup8
    dup11
    %add_const(28)
    %mstore_kernel_general_u32
    %jump(sha2_compression_loop)
sha2_compression_end:
    JUMPDEST
    // stack: num_blocks, a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], h[0]+h[64], scratch_space_addr, message_schedule_addr, i, retdest
    pop
    // stack: a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], h[0]+h[64], scratch_space_addr, message_schedule_addr, i, retdest
    %shl_const(32)
    or
    %shl_const(32)
    or
    %shl_const(32)
    or
    %shl_const(32)
    or
    %shl_const(32)
    or
    %shl_const(32)
    or
    %shl_const(32)
    or
    // stack: concat(a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], h[0]+h[64]), scratch_space_addr, message_schedule_addr, i, retdest
    swap3
    // stack: i, scratch_space_addr, message_schedule_addr, concat(a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], h[0]+h[64]), retdest
    %pop3
    // stack: sha2_result = concat(a[0]+a[64], b[0]+b[64], c[0]+c[64], d[0]+d[64], e[0]+e[64], f[0]+f[64], g[0]+g[64], h[0]+h[64]), retdest
    STOP