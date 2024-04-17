global precompile_id:
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
    %mul_const(@ID_DYNAMIC_GAS)
    PUSH @ID_STATIC_GAS
    ADD
    // stack: gas, kexit_info
    %charge_gas

    // Simply copy the call data to the parent's return data.
    %calldatasize
    DUP1 %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE)

    PUSH id_contd SWAP1

    PUSH @SEGMENT_CALLDATA
    GET_CONTEXT
    %build_address_no_offset
    // stack: SRC, size, id_contd

    PUSH @SEGMENT_RETURNDATA
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    %build_address_no_offset

    // stack: DST, SRC, size, id_contd
    %jump(memcpy_bytes)

id_contd:
    // stack: kexit_info
    %leftover_gas
    // stack: leftover_gas
    PUSH 1 // success
    %jump(terminate_common)
