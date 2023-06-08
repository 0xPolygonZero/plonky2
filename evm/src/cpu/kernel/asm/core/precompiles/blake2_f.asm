global precompile_blake2_f:
    // stack: retdest, new_ctx, (old stack)
    POP
    // stack: new_ctx, (old stack)
    %set_new_ctx_parent_pc(after_precompile)
    // stack: new_ctx, (old stack)
    DUP1
    SET_CONTEXT
    %checkpoint // Checkpoint
    %increment_call_depth
    // stack: (empty)
    PUSH 0x100000000 // = 2^32 (is_kernel = true)
    // stack: kexit_info

    PUSH blake2_f_contd
    // stack: blake2_f_contd, kexit_info

    // Load inputs from calldata memory into stack.

    %calldatasize
    // stack: calldatasize, blake2_f_contd, kexit_info
    DUP1
    // stack: calldatasize, calldatasize, blake2_f_contd, kexit_info
    %eq_const(213) ISZERO %jumpi(fault_exception)
    // stack: calldatasize, blake2_f_contd, kexit_info
    %decrement
    // stack: flag_addr=212, blake2_f_contd, kexit_info
    DUP1
    // stack: flag_addr, flag_addr, blake2_f_contd, kexit_info
    PUSH @SEGMENT_CALLDATA
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, flag_addr, flag_addr, blake2_f_contd, kexit_info
    MLOAD_GENERAL
    // stack: flag, flag_addr, blake2_f_contd, kexit_info
    DUP1
    // stack: flag, flag, flag_addr, blake2_f_contd, kexit_info
    %gt_const(1) %jumpi(fault_exception) // Check flag < 2 (flag = 0 or flag = 1)
    // stack: flag, flag_addr, blake2_f_contd, kexit_info
    SWAP1
    // stack: flag_addr, flag, blake2_f_contd, kexit_info
    %sub_const(8)
    // stack: t1_addr=flag_addr-8, flag, blake2_f_contd, kexit_info

    %stack (t1_addr) -> (@SEGMENT_CALLDATA, t1_addr, 8, t1_addr)
    // stack: @SEGMENT_CALLDATA, t1_addr, 8, t1_addr, flag, blake2_f_contd, kexit_info
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, t1_addr, 8, t1_addr, flag, blake2_f_contd, kexit_info
    %mload_packing
    // stack: t_1, t1_addr, flag, blake2_f_contd, kexit_info
    SWAP1
    // stack: t1_addr, t_1, flag, blake2_f_contd, kexit_info
    %sub_const(8)
    // stack: t0_addr=t1_addr-8, t_1, flag, blake2_f_contd, kexit_info

    %stack (t0_addr) -> (@SEGMENT_CALLDATA, t0_addr, 8, t0_addr)
    // stack: @SEGMENT_CALLDATA, t0_addr, 8, t0_addr, t_1, flag, blake2_f_contd, kexit_info
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, t0_addr, 8, t0_addr, t_1, flag, blake2_f_contd, kexit_info
    %mload_packing
    // stack: t_0, t0_addr, t_1, flag, blake2_f_contd, kexit_info
    SWAP1
    // stack: t0_addr, t_0, t_1, flag, blake2_f_contd, kexit_info
    %sub_const(128) // 16 * 8
    // stack: m0_addr=t0_addr-128, t_0, t_1, flag, blake2_f_contd, kexit_info

    %rep 16
        // stack: 68 + 8 * i, m_(i-1), ..., m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
        PUSH 8
        // stack: 8, 68 + 8 * i, m_(i-1), ..., m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
        DUP2
        // stack: 68 + 8 * i, 8, 68 + 8 * i, m_(i-1), ..., m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
        PUSH @SEGMENT_CALLDATA
        // stack: @SEGMENT_CALLDATA, 68 + 8 * i, 8, 68 + 8 * i, m_(i-1), ..., m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
        GET_CONTEXT
        // stack: ctx, @SEGMENT_CALLDATA, 68 + 8 * i, 8, 68 + 8 * i, m_(i-1), ..., m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
        %mload_packing
        // stack: m_i, 68 + 8 * i, m_(i-1), ..., m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
        SWAP1
        // stack: 68 + 8 * i, m_i, m_(i-1), ..., m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
        %add_const(8)
    %endrep
    // stack: 68 + 8 * 16 = 196, m_15, ..., m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
    %sub_const(192) // 16 * 8 (m values) + 8 * 8 (h values)
    // stack: h0_addr, m_15, ..., m_0, t_0, t_1, flag, blake2_f_contd, kexit_info

    %rep 8
        // stack: 4 + 8 * i, h_(i-1), ..., h_0, m_15..m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
        PUSH 8
        // stack: 8, 4 + 8 * i, h_(i-1), ..., h_0, m_15..m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
        DUP2
        // stack: 4 + 8 * i, 8, 4 + 8 * i, h_(i-1), ..., h_0, m_15..m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
        PUSH @SEGMENT_CALLDATA
        // stack: @SEGMENT_CALLDATA, 4 + 8 * i, 8, 4 + 8 * i, h_(i-1), ..., h_0, m_15..m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
        GET_CONTEXT
        // stack: ctx, @SEGMENT_CALLDATA, 4 + 8 * i, 8, 4 + 8 * i, h_(i-1), ..., h_0, m_15..m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
        %mload_packing
        // stack: h_i, 4 + 8 * i, h_(i-1), ..., h_0, m_15..m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
        SWAP1
        // stack: 4 + 8 * i, h_i, h_(i-1), ..., h_0, m_15..m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
        %add_const(8)
    %endrep
    // stack: 4 + 8 * 8 = 68, h_7, ..., h_0, m_15..m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
    POP

    %stack () -> (@SEGMENT_CALLDATA, 0, 4)
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 0, 4, h_7..h_0, m_15..m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
    %mload_packing
    // stack: rounds, h_7..h_0, m_15..m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
    
    DUP1
    // stack: rounds, rounds, h_7..h_0, m_15..m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
    %charge_gas
    
    // stack: rounds, h_7..h_0, m_15..m_0, t_0, t_1, flag, blake2_f_contd, kexit_info
    %jump(blake2_f)
blake2_f_contd:
    // stack: h_0', h_1', h_2', h_3', h_4', h_5', h_6', h_7', kexit_info
    // Store the result hash to the parent's return data using `mstore_unpacking`.

    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 32)
    PUSH 0
    // stack: addr_0=0, h_0', h_1', h_2', h_3', h_4', h_5', h_6', h_7', kexit_info
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    // stack: parent_ctx, addr_0=0, h_0', h_1', h_2', h_3', h_4', h_5', h_6', h_7', kexit_info

    %rep 8
        // stack: parent_ctx, addr_i, h_i', ..., h_7', kexit_info
        %stack (ctx, addr, h_i) -> (ctx, @SEGMENT_RETURNDATA, addr, h_i, 4, addr, ctx)
        // stack: parent_ctx, @SEGMENT_RETURNDATA, addr_i, h_i', 4, addr_i, parent_ctx, h_(i+1)', ..., h_7', kexit_info
        %mstore_unpacking
        // stack: addr_i, parent_ctx, h_(i+1)', ..., h_7', kexit_info
        %add_const(4)
        // stack: addr_(i+1), parent_ctx, h_(i+1)', ..., h_7', kexit_info
        SWAP1
        // stack: parent_ctx, addr_(i+1), h_(i+1)', ..., h_7', kexit_info
    %endrep

    // stack: kexit_info    
    %jump(pop_and_return_success)
