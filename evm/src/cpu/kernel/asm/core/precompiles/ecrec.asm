global precompile_ecrec:
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

    %charge_gas_const(@ECREC_GAS)

    // Load hash, v, r, s from the call data using `mload_packing`.
    PUSH ecrec_return
    // stack: ecrec_return, kexit_info
    %stack () -> (@SEGMENT_CALLDATA, 96, 32)
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 96, 32, ecrec_return, kexit_info
    %build_address
    %mload_packing
    // stack: s, ecrec_return, kexit_info
    %stack () -> (@SEGMENT_CALLDATA, 64, 32)
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 64, 32, s, ecrec_return, kexit_info
    %build_address
    %mload_packing
    // stack: r, s, ecrec_return, kexit_info
    %stack () -> (@SEGMENT_CALLDATA, 32, 32)
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 32, 32, r, s, ecrec_return, kexit_info
    %build_address
    %mload_packing
    // stack: v, r, s, ecrec_return, kexit_info
    %stack () -> (@SEGMENT_CALLDATA, 32)
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 32, v, r, s, ecrec_return, kexit_info
    %build_address_no_offset
    %mload_packing
    // stack: hash, v, r, s, ecrec_return, kexit_info
    %jump(ecrecover)
ecrec_return:
    // stack: address, kexit_info
    DUP1 %eq_const(@U256_MAX) %jumpi(ecrec_bad_input) // ecrecover returns U256_MAX on bad input.

    // Store the result address to the parent's return data using `mstore_unpacking`.
    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 32)
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    %stack (parent_ctx, address) -> (parent_ctx, @SEGMENT_RETURNDATA, address, 32, pop_and_return_success)
    %build_address_no_offset
    %jump(mstore_unpacking)

// On bad input, return empty return data but still return success.
ecrec_bad_input:
    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 0)
    %jump(pop_and_return_success)
