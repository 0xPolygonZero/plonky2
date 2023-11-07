global precompile_sha256:
    // stack: address, retdest, new_ctx, (old stack)
    %pop2
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

    %calldatasize
    %num_bytes_to_num_words
    // stack: data_words_len, kexit_info
    %mul_const(@SHA256_DYNAMIC_GAS)
    PUSH @SHA256_STATIC_GAS
    ADD
    // stack: gas, kexit_info
    %charge_gas

    // TODO: fix this
    %zero_out_kernel_general

    // Copy the call data to the kernel general segment (sha2 expects it there) and call sha2.
    %calldatasize
    GET_CONTEXT
    // stack: ctx, size

    // The next block of code is equivalent to the following %stack macro call
    // (unfortunately the macro call takes too long to expand dynamically).
    //
    //    %stack (ctx, size) ->
    //        (
    //        ctx, @SEGMENT_KERNEL_GENERAL, 1, // DST
    //        ctx, @SEGMENT_CALLDATA, 0,     // SRC
    //        size, sha2,                    // count, retdest
    //        0, size, sha256_contd          // sha2 input: virt, num_bytes, retdest
    //        )
    //
    PUSH 0
    PUSH sha2
    DUP4
    PUSH 0
    PUSH @SEGMENT_CALLDATA
    PUSH sha256_contd
    SWAP7
    SWAP6
    PUSH 1
    PUSH @SEGMENT_KERNEL_GENERAL
    DUP3

    %jump(memcpy)

sha256_contd:
    // stack: hash, kexit_info
    // Store the result hash to the parent's return data using `mstore_unpacking`.
    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 32)
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    %stack (parent_ctx, hash) -> (parent_ctx, @SEGMENT_RETURNDATA, 0, 32, hash, pop_and_return_success)
    %jump(mstore_unpacking)
