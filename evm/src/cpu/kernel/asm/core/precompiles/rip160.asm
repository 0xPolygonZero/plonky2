global precompile_rip160:
    // stack: address, retdest, new_ctx, (old stack)
    %pop2
    // stack: new_ctx, (old stack)
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

    // Copy the call data to the kernel general segment (ripemd expects it there) and call ripemd.
    %calldatasize
    GET_CONTEXT

    // The next block of code is equivalent to the following %stack macro call
    // (unfortunately the macro call takes too long to expand dynamically).
    //
    //    %stack (ctx, size) ->
    //        (
    //        0, @SEGMENT_KERNEL_GENERAL, 200, // DST
    //        ctx, @SEGMENT_CALLDATA, 0,       // SRC
    //        size, ripemd,                    // count, retdest
    //        200, size, rip160_contd          // ripemd input: virt, num_bytes, retdest
    //        )
    PUSH 200
    PUSH ripemd
    DUP4
    PUSH 0
    PUSH @SEGMENT_CALLDATA
    PUSH rip160_contd
    SWAP7
    SWAP6
    PUSH 200
    PUSH @SEGMENT_KERNEL_GENERAL
    PUSH 0

    %jump(memcpy)

rip160_contd:
    // stack: hash, kexit_info
    // Store the result hash to the parent's return data using `mstore_unpacking`.
    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 32)
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    %stack (parent_ctx, hash) -> (parent_ctx, @SEGMENT_RETURNDATA, 0, hash, 32, pop_and_return_success)
    %jump(mstore_unpacking)
