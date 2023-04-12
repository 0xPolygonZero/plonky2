global precompile_sha256:
    // How do we pay gas in this "new context"?
    // stack: address, %%after, new_ctx, kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size
    %stack (address, retdest, address, gas, kexit_info, value, args_offset, args_size, ret_offset, ret_size) ->
        (args_size, kexit_info, args_offset, args_size, ret_offset, ret_size)

    %num_bytes_to_num_words
    // stack: data_words_len
    %mul_const(@SHA256_DYNAMIC_GAS)
    PUSH @SHA256_STATIC_GAS
    ADD
    %charge_gas
    %stack (kexit_info, args_offset, args_size, ret_offset, ret_size) ->
        (args_offset, args_size, ret_offset, ret_size, kexit_info)

    %zero_out_kernel_general

    GET_CONTEXT
    %stack (ctx, args_offset, args_size) ->
        (
        0, @SEGMENT_KERNEL_GENERAL, 1,              // DST
        ctx, @SEGMENT_MAIN_MEMORY, args_offset,     // SRC
        args_size, sha2,                            // count, retdest
        0, args_size, sha256_contd                  // sha2 input: virt, num_bytes, retdest
        )
    %jump(memcpy)

sha256_contd:
    // stack: hash
    GET_CONTEXT
    %stack (ctx, hash) -> (ctx, @SEGMENT_RETURNDATA, 0, hash, 32, sha256_contd_bis)
    %jump(mstore_unpacking)
global sha256_contd_bis:
    POP
    // stack: ret_offset, ret_size, kexit_info
    %jump(after_precompile)
