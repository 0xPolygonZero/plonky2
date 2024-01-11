global blake2_compression:
    // stack: retdest
    PUSH 0
    // stack: cur_block = 0, retdest
    PUSH compression_loop
    // stack: compression_loop, cur_block, retdest
    %jump(blake2_initial_hash_value)
compression_loop:
    // stack: h_0, ..., h_7, cur_block, retdest
    
    // Store the hash values.
    %blake2_hash_value_addr
    // stack: addr, h_0, ..., h_7, cur_block, retdest
    %rep 8
        SWAP1
        DUP2
        %mstore_current_general
        %increment
    %endrep

    // stack: addr, cur_block, retdest
    POP
    // stack: cur_block, retdest
    PUSH 1
    PUSH 0
    %mload_current_general
    // stack: num_blocks, 1, cur_block, retdest
    SUB
    // stack: num_blocks - 1, cur_block, retdest
    DUP2
    // stack: cur_block, num_blocks - 1, cur_block, retdest
    EQ
    // stack: is_last_block, cur_block, retdest
    SWAP1
    // stack: cur_block, is_last_block, retdest
    PUSH 1
    %mload_current_general
    // stack: num_bytes, cur_block, is_last_block, retdest

    // Calculate t counter value.
    DUP3
    // stack: is_last_block, num_bytes, cur_block, is_last_block, retdest
    MUL
    // stack: is_last_block * num_bytes, cur_block, is_last_block, retdest
    DUP2
    // stack: cur_block, is_last_block * num_bytes, cur_block, is_last_block, retdest
    %increment
    %block_size
    // stack: (cur_block + 1) * 128, is_last_block * num_bytes, cur_block, is_last_block, retdest
    DUP4
    // stack: is_last_block, (cur_block + 1) * 128, is_last_block * num_bytes, cur_block, is_last_block, retdest
    ISZERO
    // stack: not_last_block, (cur_block + 1) * 128, is_last_block * num_bytes, cur_block, is_last_block, retdest
    MUL
    // stack: not_last_block * ((cur_block + 1) * 128), is_last_block * num_bytes, cur_block, is_last_block, retdest
    ADD
    // stack: t = not_last_block * ((cur_block + 1) * 128) + is_last_block * num_bytes, cur_block, is_last_block, retdest
    SWAP1
    // stack: cur_block, t, is_last_block, retdest
    DUP1
    // stack: cur_block, cur_block, t, is_last_block, retdest
    %block_size
    %add_const(2)
    // stack: cur_block_start_byte, t, cur_block, is_last_block, retdest

    // Copy the message from the input space to the message working space.
    %blake2_message_addr
    // stack: message_addr, cur_block_start_byte, t, cur_block, is_last_block, retdest
    %rep 16
        // stack: cur_message_addr, cur_block_byte, ...
        DUP2
        // stack: cur_block_byte, cur_message_addr, cur_block_byte, ...
        %mload_current_general_u64_LE
        // stack: m_i, cur_message_addr, cur_block_byte, ...
        DUP2
        // stack: cur_message_addr, m_i, cur_message_addr, cur_block_byte, ...
        %mstore_current_general
        // stack: cur_message_addr, cur_block_byte, ...
        %increment
        // stack: cur_message_addr + 1, cur_block_byte, ...
        SWAP1
        // stack: cur_block_byte, cur_message_addr + 1, ...
        %add_const(8)
        // stack: cur_block_byte + 8, cur_message_addr + 1, ...
        SWAP1
        // stack: cur_message_addr + 1, cur_block_byte + 8, ...
    %endrep
    // stack: end_message_addr, end_block_start_byte, t, cur_block, is_last_block, retdest
    %pop2
    // stack: t, cur_block, is_last_block, retdest
    SWAP1
    // stack: cur_block, t, is_last_block, retdest
    SWAP2
    // stack: is_last_block, t, cur_block, retdest
    %mul_const(0xFFFFFFFFFFFFFFFF)
    // stack: invert_if_last_block, t, cur_block, retdest
    %blake2_hash_value_addr
    %add_const(7)
    %rep 8
        // stack: addr, ...
        DUP1
        // stack: addr, addr, ...
        %mload_current_general
        // stack: val, addr, ...
        SWAP1
        // stack: addr, val, ...
        %decrement
    %endrep
    // stack: addr, h_0, ..., h_7, invert_if_last_block, t, cur_block, retdest
    POP
    // stack: h_0, ..., h_7, invert_if_last_block, t, cur_block, retdest

    // Store the initial 16 values of the internal state.
    %blake2_internal_state_addr
    // stack: start, h_0, ..., h_7, invert_if_last_block, t, cur_block, retdest

    // First eight words of the internal state: current hash value h_0, ..., h_7.
    %rep 8
        SWAP1
        DUP2
        %mstore_current_general
        %increment
    %endrep
    // stack: start + 8, invert_if_last_block, t, cur_block, retdest

    // Next four values of the internal state: first four IV values.
    PUSH 0
    // stack: 0, start + 8, invert_if_last_block, t, cur_block, retdest
    %rep 4
        // stack: i, loc, ...
        DUP1
        // stack: i, i, loc, ...
        %blake2_iv
        // stack: IV_i, i, loc, ...
        DUP3
        // stack: loc, IV_i, i, loc, ...
        %mstore_current_general
        // stack: i, loc, ...
        %increment
        SWAP1
        %increment
        SWAP1
        // stack: i + 1, loc + 1,...
    %endrep
    // stack: 4, start + 12, invert_if_last_block, t, cur_block, retdest
    %stack (i, loc, inv, last, t) -> (t, t, i, loc, inv, last)
    // stack: t, t, 4, start + 12, invert_if_last_block, cur_block, retdest
    %shr_const(64)
    // stack: t_hi = t >> 64, t, 4, start + 12, invert_if_last_block, cur_block, retdest
    SWAP1
    // stack: t, t_hi, 4, start + 12, invert_if_last_block, cur_block, retdest
    %mod_const(0x10000000000000000)
    // stack: t_lo = t % (1 << 64), t_hi, 4, start + 12, invert_if_last_block, cur_block, retdest
    %stack (t_lo, t_hi, i, loc, inv) -> (i, loc, t_lo, t_hi, inv, 0)
    // stack: 4, start + 12, t_lo, t_hi, invert_if_last_block, 0, cur_block, retdest

    // Last four values of the internal state: last four IV values, XOR'd with
    // the values (t % 2**64, t >> 64, invert_if, 0).
    %rep 4
        // stack: i, loc, val, next_val,...
        DUP1
        // stack: i, i, loc, val, next_val,...
        %blake2_iv
        // stack: IV_i, i, loc, val, next_val,...
        DUP4
        // stack: val, IV_i, i, loc, val, next_val,...
        XOR
        // stack: val ^ IV_i, i, loc, val, next_val,...
        DUP3
        // stack: loc, val ^ IV_i, i, loc, val, next_val,...
        %mstore_current_general
        // stack: i, loc, val, next_val,...
        %increment
        // stack: i + 1, loc, val, next_val,...
        SWAP2
        // stack: val, loc, i + 1, next_val,...
        POP
        // stack: loc, i + 1, next_val,...
        %increment
        // stack: loc + 1, i + 1, next_val,...
        SWAP1
        // stack: i + 1, loc + 1, next_val,...
    %endrep
    // stack: 8, loc + 16, cur_block, retdest
    %pop2
    // stack: cur_block, retdest

    // Run 12 rounds of G functions.
    PUSH g_functions_return
    // stack: g_functions_return, cur_block, retdest
    PUSH 12
    %blake2_internal_state_addr
    // stack: start, 12, g_functions_return, cur_block, retdest
    PUSH 0
    // stack: current_round=0, start, 12, g_functions_return, cur_block, retdest
    %jump(run_rounds_g_function)
g_functions_return:
    // Finalize hash value.
    // stack: cur_block, retdest
    PUSH hash_generate_return
    // stack: hash_generate_return, cur_block, retdest
    %jump(blake2_generate_all_hash_values)
hash_generate_return:
    // stack: h_0', h_1', h_2', h_3', h_4', h_5', h_6', h_7', cur_block, retdest
    DUP9
    // stack: cur_block, h_0', h_1', h_2', h_3', h_4', h_5', h_6', h_7', cur_block, retdest
    %increment
    // stack: cur_block + 1, h_0', h_1', h_2', h_3', h_4', h_5', h_6', h_7', cur_block, retdest
    SWAP9
    // stack: cur_block, h_0', h_1', h_2', h_3', h_4', h_5', h_6', h_7', cur_block + 1, retdest
    %increment
    // stack: cur_block + 1, h_0', h_1', h_2', h_3', h_4', h_5', h_6', h_7', cur_block + 1, retdest
    PUSH 0
    %mload_current_general
    // stack: num_blocks, cur_block + 1, h_0', h_1', h_2', h_3', h_4', h_5', h_6', h_7', cur_block + 1, retdest
    GT
    // stack: not_last_block, h_0', h_1', h_2', h_3', h_4', h_5', h_6', h_7', cur_block + 1, retdest
    %jumpi(compression_loop)
compression_end:
    // stack: h_0', h_1', h_2', h_3', h_4', h_5', h_6', h_7', cur_block + 1, retdest

    // Invert the bytes of each hash value.
    %reverse_bytes_u64
    // stack: h_0'', h_1', h_2', h_3', h_4', h_5', h_6', h_7', cur_block + 1, retdest
    SWAP1
    // stack: h_1', h_0'', h_2', h_3', h_4', h_5', h_6', h_7', cur_block + 1, retdest
    %reverse_bytes_u64
    // stack: h_1'', h_0'', h_2', h_3', h_4', h_5', h_6', h_7', cur_block + 1, retdest
    SWAP2
    // stack: h_2', h_0'', h_1'', h_3', h_4', h_5', h_6', h_7', cur_block + 1, retdest
    %reverse_bytes_u64
    // stack: h_2'', h_0'', h_1'', h_3', h_4', h_5', h_6', h_7', cur_block + 1, retdest
    SWAP3
    // stack: h_3', h_0'', h_1'', h_2'', h_4', h_5', h_6', h_7', cur_block + 1, retdest
    %reverse_bytes_u64
    // stack: h_3'', h_0'', h_1'', h_2'', h_4', h_5', h_6', h_7', cur_block + 1, retdest
    SWAP4
    // stack: h_4', h_0'', h_1'', h_2'', h_3'', h_5', h_6', h_7', cur_block + 1, retdest
    %reverse_bytes_u64
    // stack: h_4'', h_0'', h_1'', h_2'', h_3'', h_5', h_6', h_7', cur_block + 1, retdest
    SWAP5
    // stack: h_5', h_0'', h_1'', h_2'', h_3'', h_4'', h_6', h_7', cur_block + 1, retdest
    %reverse_bytes_u64
    // stack: h_5'', h_0'', h_1'', h_2'', h_3'', h_4'', h_6', h_7', cur_block + 1, retdest
    SWAP6
    // stack: h_6', h_0'', h_1'', h_2'', h_3'', h_4'', h_5'', h_7', cur_block + 1, retdest
    %reverse_bytes_u64
    // stack: h_6'', h_0'', h_1'', h_2'', h_3'', h_4'', h_5'', h_7', cur_block + 1, retdest
    SWAP7
    // stack: h_7', h_0'', h_1'', h_2'', h_3'', h_4'', h_5'', h_6'', cur_block + 1, retdest
    %reverse_bytes_u64
    // stack: h_7'', h_0'', h_1'', h_2'', h_3'', h_4'', h_5'', h_6'', cur_block + 1, retdest
    %stack (h_7, h_s: 7) -> (h_s, h_7)
    // stack: h_0'', h_1'', h_2'', h_3'', h_4'', h_5'', h_6'', h_7'', cur_block + 1, retdest

    // Combine hash values.
    %u64s_to_u256
    // stack: h_0'' || h_1'' || h_2'' || h_3'', h_4'', h_5'', h_6'', h_7'', cur_block + 1, retdest
    %stack (first, second: 4, cur) -> (second, first)
    // stack: h_4'', h_5'', h_6'', h_7'', h_0'' || h_1'' || h_2'' || h_3'', retdest
    %u64s_to_u256
    // stack: hash_second = h_4'' || h_5'' || h_6'' || h_7'', hash_first = h_0'' || h_1'' || h_2'' || h_3'', retdest
    %stack (second, first, ret) -> (ret, second, first)
    // stack: retdest, hash_first, hash_second
    JUMP
