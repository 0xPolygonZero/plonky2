global precompile_rip160:
    %stack (address, retdest, new_ctx, kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size) ->
        (new_ctx, kexit_info, ret_offset, ret_size)
    DUP1
    SET_CONTEXT
    // stack: (empty)
    PUSH 0x100000000 // = 2^32 (is_kernel = true)
    // stack: kexit_info

    %calldatasize
    %num_bytes_to_num_words
    // stack: data_words_len, kexit_info
    %mul_const(@RIP160_DYNAMIC_GAS)
    PUSH @RIP160_STATIC_GAS
    ADD
    // stack: gas, kexit_info
    %charge_gas

    %zero_out_kernel_general

    %calldatasize
    GET_CONTEXT
    %stack (ctx, size) ->
        (
        0, @SEGMENT_KERNEL_GENERAL, 200, // DST
        ctx, @SEGMENT_CALLDATA, 0,       // SRC
        size, ripemd,                    // count, retdest
        200, size, rip160_contd          // ripemd input: virt, num_bytes, retdest
        )
    %jump(memcpy)

global rip160_contd:
    // stack: hash, kexit_info
    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 32)
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    %stack (parent_ctx, hash) -> (parent_ctx, @SEGMENT_RETURNDATA, 0, hash, 32, pop_and_return_success)
    %jump(mstore_unpacking)
