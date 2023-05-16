global blake2_f:
    // stack: rounds, h0...h7, m0...m15, t0, t1, flag, retdest

    // Store the hash values.
    %blake2_hash_value_addr
    // stack: addr, rounds, h0...h7, m0...m15, t0, t1, flag, retdest
    %rep 8
        // stack: addr, rounds, h_i, ...
        %stack (addr, rounds, h_i) -> (addr, h_i, addr, rounds)
        // stack: addr, h_i, addr, rounds, ...
        %mstore_current_general
        %increment
    %endrep

    // stack: addr, rounds, m0...m15, t0, t1, flag, retdest
    POP
    // stack: rounds, m0...m15, t0, t1, flag, retdest

    // Save the message to the message working space.
    %blake2_message_addr
    // stack: message_addr, rounds, m0...m15, t0, t1, flag, retdest
    %rep 16
        // stack: message_addr, rounds, m_i, ...
        %stack (message_addr, rounds, m_i) -> (message_addr, m_i, message_addr, rounds)
        // stack: message_addr, m_i, message_addr, rounds, ...
        %mstore_current_general
        %increment
    %endrep

    // stack: message_addr, rounds, t0, t1, flag, retdest
    POP
    // stack: rounds, t0, t1, flag, retdest

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
    // stack: addr, h_0, ..., h_7, rounds, t0, t1, flag, retdest
    POP
    // stack: h_0, ..., h_7, rounds, t0, t1, flag, retdest

    // Store the initial 16 values of the internal state.
    %blake2_internal_state_addr
    // stack: start, h_0, ..., h_7, rounds, t0, t1, flag, retdest

    // First eight words of the internal state: current hash value h_0, ..., h_7.
    %rep 8
        SWAP1
        DUP2
        %mstore_current_general
        %increment
    %endrep
    // stack: start + 8, rounds, t0, t1, flag, retdest

    // Next four values of the internal state: first four IV values.
    PUSH 0
    // stack: 0, start + 8, rounds, t0, t1, flag, retdest
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
    // stack: 4, start + 12, rounds, t0, t1, flag, retdest
    POP
    // stack: start + 12, rounds, t0, t1, flag, retdest
    SWAP4
    // stack: flag, rounds, t0, t1, start + 12, retdest
    %mul_const(0xFFFFFFFFFFFFFFFF)
    // stack: invert_if_flag, rounds, t0, t1, start + 12, retdest
    %stack (inv, r, t0, t1, s) -> (4, s, t0, t1, inv, 0, r)
    // stack: 4, start + 12, t0, t1, invert_if_flag, 0, rounds, retdest

    // Last four values of the internal state: last four IV values, XOR'd with
    // the values (t0, t1, invert_if_flag, 0).
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
    // stack: 8, start + 16, rounds, retdest
    %pop2
    // stack: rounds, retdest

    // Run rounds of G functions.
    PUSH g_functions_return
    // stack: g_functions_return, rounds, retdest
    SWAP1
    // stack: rounds, g_functions_return, retdest
    %blake2_internal_state_addr
    // stack: start, rounds, g_functions_return, retdest
    PUSH 0
    // stack: current_round=0, start, rounds, g_functions_return, retdest
    %jump(run_rounds_g_function)
g_functions_return:
    // Finalize hash value.
    // stack: retdest
    PUSH hash_generate_return
    // stack: hash_generate_return, retdest
    %jump(blake2_generate_all_hash_values)
hash_generate_return:
    // stack: h_0', h_1', h_2', h_3', h_4', h_5', h_6', h_7', retdest
    %stack (h: 8, retdest) -> (retdest, h)
    // stack: retdest, h_0', h_1', h_2', h_3', h_4', h_5', h_6', h_7'
    JUMP
