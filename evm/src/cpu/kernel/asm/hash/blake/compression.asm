%macro blake_initial_hash_value
    %blake_iv_i(7)
    %blake_iv_i(6)
    %blake_iv_i(5)
    %blake_iv_i(4)
    %blake_iv_i(3)
    %blake_iv_i(2)
    %blake_iv_i(1)
    // stack: IV_1, IV_2, IV_3, IV_4, IV_5, IV_6, IV_7
    PUSH 0x01010040 // params: key = 00, digest_size = 64 = 0x40
    %blake_iv_i(0)
    XOR
    // stack: IV_0 ^ params, IV_1, IV_2, IV_3, IV_4, IV_5, IV_6, IV_7
%endmacro

%macro blake_hash_value_addr
    PUSH 0
    // stack: 0
    %mload_kernel_general
    // stack: num_blocks
    %mul_const(128)
    %increment
    // stack: num_bytes+1
%endmacro

%macro blake_internal_state_addr
    %blake_hash_value_addr
    %add_const(8)
%endmacro

%macro blake_message_addr
    %blake_internal_state_addr
    %add_const(16)
%endmacro

global blake_compression:
    // stack: retdest
    %stack () -> (0, 0, 0)
    // stack: cur_block = 0, t_0 = 0, t_1 = 0, retdest

    // TODO: load %blake_initial_hash_value and store to blake_hash_value_addr

compression_loop:
    // stack: cur_block, t_0, t_1, retdest
    PUSH 0
    %mload_kernel_general
    // stack: num_blocks, cur_block, t_0, t_1, retdest
    %decrement
    // stack: num_blocks - 1, cur_block, t_0, t_1, retdest
    DUP2
    // stack: cur_block, num_blocks - 1, cur_block, t_0, t_1, retdest
    EQ
    // stack: is_last_block, cur_block, t_0, t_1, retdest
    SWAP1
    // stack: cur_block, is_last_block, t_0, t_1, retdest
    %mul_const(128)
    %increment
    // stack: cur_block_start_byte, is_last_block, t_0, t_1, retdest
    %blake_message_addr
    // stack: message_addr, cur_block_start_byte, is_last_block, t_0, t_1, retdest
    %rep 16
        // stack: cur_message_addr, cur_block_byte, ...
        DUP2
        // stack: cur_block_byte, cur_message_addr, cur_block_byte, ...
        %mload_blake_word
        // stack: m_i, cur_message_addr, cur_block_byte, ...
        DUP2
        // stack: cur_message_addr, m_i, cur_message_addr, cur_block_byte, ...
        %mstore_kernel_general
        // stack: cur_message_addr, cur_block_byte, ...
        %increment
        // stack: cur_message_addr + 1, cur_block_byte, ...
        SWAP1
        // stack: cur_block_byte, cur_message_addr + 1, ...
        %add_const(64)
        // stack: cur_block_byte + 64, cur_message_addr + 1, ...
        SWAP1
        // stack: cur_message_addr + 1, cur_block_byte + 64, ...
    %endrep
    // stack: end_message_addr, end_block_start_byte, is_last_block, t_0, t_1, retdest
    POP
    POP
    // stack: is_last_block, t_0, t_1, retdest
    %mul_const(0xFFFFFFFF)
    %stack (l, t0, t1) -> (t0, t1, l, 0)
    // stack: t_0, t_1, invert_if_last_block, 0, retdest
    // TODO: LOAD from %blake_hash_value_addr
    // stack: h_0, ..., h_7, t_0, t_1, invert_if_last_block, 0, retdest
    %blake_internal_state_addr
    // stack: start, h_0, ..., h_7, t_0, t_1, invert_if_last_block, 0, retdest
    // First eight words of compression state: current state h_0, ..., h_7.
    %rep 8
        SWAP1
        DUP2
        %mstore_kernel_general
        %increment
    %endrep
    // stack: start + 8, t_0, t_1, invert_if_last_block, 0, retdest
    PUSH 0
    // stack: 0, start + 8, t_0, t_1, invert_if_last_block, 0, retdest
    %rep 4
        // stack: i, loc, ...
        DUP2
        DUP2
        // stack: i, loc, i, loc,...
        %blake_iv
        // stack: IV_i, loc, i, loc,...
        SWAP1
        // stack: loc, IV_i, i, loc,...
        %mstore_kernel_general
        // stack: i, loc,...
        %increment
        SWAP1
        %increment
        SWAP1
        // stack: i + 1, loc + 1,...
    %endrep
    %rep 4
        // stack: i, loc, val, next_val, next_val,...
        %stack (i, loc, val) -> (i, val, loc, i, loc)
        // stack: i, val, loc, i, loc, next_val,...
        %blake_iv
        // stack: IV_i, val, loc, i, loc, next_val,...
        XOR
        // stack: val ^ IV_i, loc, i, loc, next_val,...
        SWAP1
        // stack: loc, val ^ IV_i, i, loc, next_val,...
        %mstore_kernel_general
        // stack: i, loc, next_val,...
        %increment
        SWAP1
        %increment
        SWAP1
        // stack: i + 1, loc + 1, next_val,...
    %endrep
    // stack: 8, loc + 16
    POP
    POP
    // stack: (empty)
    %blake_internal_state_addr
    // stack: start
    PUSH 0
    // stack: round=0, start
    %rep 12
        // stack: round, start
        %call_blake_g_function(0, 4, 8, 12, 0, 1)
        %call_blake_g_function(1, 5, 9, 13, 2, 3)
        %call_blake_g_function(2, 6, 10, 14, 4, 5)
        %call_blake_g_function(3, 7, 11, 15, 6, 7)
        %call_blake_g_function(0, 5, 10, 15, 8, 9)
        %call_blake_g_function(1, 6, 11, 12, 10, 11)
        %call_blake_g_function(2, 7, 8, 13, 12, 13)
        %call_blake_g_function(3, 4, 9, 14, 14, 15)
        // stack: round, start
        %increment
        // stack: round + 1, start
    %endrep
    