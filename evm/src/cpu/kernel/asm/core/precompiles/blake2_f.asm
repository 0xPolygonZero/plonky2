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
    GET_CONTEXT
    // stack: ctx, size

    // TODO: change
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
