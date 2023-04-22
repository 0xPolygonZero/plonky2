global precompile_blake2_f:
    // stack: address, retdest, new_ctx, kexit_info, ret_offset, ret_size
    %pop2
    // stack: new_ctx, kexit_info, ret_offset, ret_size
    DUP1
    SET_CONTEXT
    // stack: (empty)
    PUSH 0x100000000 // = 2^32 (is_kernel = true)
    // stack: kexit_info

    // get various inputs out of SEGMENT_CALLDATA

    // charge gas (based on rounds)

    // Copy the call data to the kernel general segment (blake2b expects it there) and call blake2b.
    %calldatasize

    // stack: size
    %stack () -> (@SEGMENT_CALLDATA, 0, 4)
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 0, 4, size
    %mload_packing
    // stack: rounds, size

    PUSH 4
    %rep 8
        // stack: 4 + 8 * i, h_(i-1), ..., h_0, rounds, size
        PUSH 8
        // stack: 8, 4 + 8 * i, h_(i-1), ..., h_0, rounds, size
        DUP2
        // stack: 4 + 8 * i, 8, 4 + 8 * i, h_(i-1), ..., h_0, rounds, size
        PUSH @SEGMENT_CALLDATA
        // stack: @SEGMENT_CALLDATA, 4 + 8 * i, 8, 4 + 8 * i, h_(i-1), ..., h_0, rounds, size
        GET_CONTEXT
        // stack: ctx, @SEGMENT_CALLDATA, 4 + 8 * i, 8, 4 + 8 * i, h_(i-1), ..., h_0, rounds, size
        %mload_packing
        // stack: h_i, 4 + 8 * i, h_(i-1), ..., h_0, rounds, size
        SWAP1
        // stack: 4 + 8 * i, h_i, h_(i-1), ..., h_0, rounds, size
        %add_const(8)
    %endrep
    // stack: 4 + 8 * 8 = 68, h_7, ..., h_0, rounds, size
    
    %rep 16
        // stack: 68 + 8 * i, m_(i-1), ..., m_0, h_7..h_0, rounds, size
        PUSH 8
        // stack: 8, 68 + 8 * i, m_(i-1), ..., m_0, h_7..h_0, rounds, size
        DUP2
        // stack: 68 + 8 * i, 8, 68 + 8 * i, m_(i-1), ..., m_0, h_7..h_0, rounds, size
        PUSH @SEGMENT_CALLDATA
        // stack: @SEGMENT_CALLDATA, 68 + 8 * i, 8, 68 + 8 * i, m_(i-1), ..., m_0, h_7..h_0, rounds, size
        GET_CONTEXT
        // stack: ctx, @SEGMENT_CALLDATA, 68 + 8 * i, 8, 68 + 8 * i, m_(i-1), ..., m_0, h_7..h_0, rounds, size
        %mload_packing
        // stack: m_i, 68 + 8 * i, m_(i-1), ..., m_0, h_7..h_0, rounds, size
        SWAP1
        // stack: 68 + 8 * i, m_i, m_(i-1), ..., m_0, h_7..h_0, rounds, size
        %add_const(8)
    %endrep
    // stack: 68 + 8 * 16 = 196, m_15, ..., m_0, h_7..h_0, rounds, size

    %stack (offset) -> (@SEGMENT_CALLDATA, offset, 8, offset)
    // stack: @SEGMENT_CALLDATA, 196, 8, 196, m_15..m_0, h_7..h_0, rounds, size
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 196, 8, 196, m_15..m_0, h_7..h_0, rounds, size
    %mload_packing
    // stack: t_0, 196, m_15..m_0, h_7..h_0, rounds, size
    SWAP1
    // stack: 196, t_0, m_15..m_0, h_7..h_0, rounds, size
    %add_const(8)
    // stack: 204, t_0, m_15..m_0, h_7..h_0, rounds, size

    %stack (offset) -> (@SEGMENT_CALLDATA, offset, 8, offset)
    // stack: @SEGMENT_CALLDATA, 204, 8, 204, t_0, m_15..m_0, h_7..h_0, rounds, size
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 204, 8, 204, t_0, m_15..m_0, h_7..h_0, rounds, size
    %mload_packing
    // stack: t_1, 204, t_0, m_15..m_0, h_7..h_0, rounds, size
    SWAP1
    // stack: 204, t_1, t_0, m_15..m_0, h_7..h_0, rounds, size
    %add_const(8)
    // stack: 212, t_1, t_0, m_15..m_0, h_7..h_0, rounds, size

    PUSH @SEGMENT_CALLDATA
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 212, t_1, t_0, m_15..m_0, h_7..h_0, rounds, size
    MLOAD_GENERAL
    // stack: f, t_1, t_0, m_15..m_0, h_7..h_0, rounds, size


    



    // The next block of code is equivalent to the following %stack macro call
    // (unfortunately the macro call takes too long to expand dynamically).
    //
    //    %stack (ctx, size) ->
    //        (
    //        0, @SEGMENT_KERNEL_GENERAL, 1, // DST
    //        ctx, @SEGMENT_CALLDATA, 0,     // SRC
    //        size, blake2_f,                    // count, retdest
    //        0, size, blake2_f_contd          // blake2b input: virt, num_bytes, retdest
    //        )
    //
    PUSH 0
    PUSH blake2_f
    DUP4
    PUSH 0
    PUSH @SEGMENT_CALLDATA
    PUSH blake2_f_contd
    SWAP7
    SWAP6
    PUSH 1
    PUSH @SEGMENT_KERNEL_GENERAL
    PUSH 0

    %jump(memcpy)

blake2_f_contd:
    // stack: hash, kexit_info
    // Store the result hash to the parent's return data using `mstore_unpacking`.



    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 32)
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    %stack (parent_ctx, hash) -> (parent_ctx, @SEGMENT_RETURNDATA, 0, hash, 32, pop_and_return_success)
    %jump(mstore_unpacking)
