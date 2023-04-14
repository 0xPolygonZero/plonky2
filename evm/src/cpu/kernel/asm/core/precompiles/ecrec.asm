global precompile_ecrec:
    // stack: address, retdest, new_ctx, kexit_info, ret_offset, ret_size
    %pop2
    // stack: new_ctx, kexit_info, ret_offset, ret_size
    DUP1
    SET_CONTEXT
    // stack: (empty)
    PUSH 0x100000000 // = 2^32 (is_kernel = true)
    // stack: kexit_info

    %charge_gas_const(@ECREC_GAS)

    // Load hash, v, r, s from the call data using `mload_packing`.
    GET_CONTEXT
    %stack (ctx, kexit_info) -> (ctx, @SEGMENT_CALLDATA, 96, 32, ecrec_contd, kexit_info)
    %jump(mload_packing)
ecrec_contd:
    GET_CONTEXT
    %stack (ctx, s, kexit_info) -> (ctx, @SEGMENT_CALLDATA, 64, 32, ecrec_contd2, s, kexit_info)
    %jump(mload_packing)
ecrec_contd2:
    GET_CONTEXT
    %stack (ctx, r, s, kexit_info) -> (ctx, @SEGMENT_CALLDATA, 32, 32, ecrec_contd3, r, s, kexit_info)
    %jump(mload_packing)
ecrec_contd3:
    GET_CONTEXT
    %stack (ctx, v, r, s, kexit_info) -> (ctx, @SEGMENT_CALLDATA, 0, 32, ecrec_contd4, v, r, s, kexit_info)
    %jump(mload_packing)
ecrec_contd4:
    %stack (hash, v, r, s, kexit_info) -> (hash, v, r, s, ecrec_contd5, kexit_info)
    %jump(ecrecover)
ecrec_contd5:
    // stack: address, kexit_info
    DUP1 %eq_const(@U256_MAX) %jumpi(ecrec_bad_input) // ecrecover returns U256_MAX on bad input.

    // Store the result address to the parent's return data using `mstore_unpacking`.
    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 32)
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    %stack (parent_ctx, address) -> (parent_ctx, @SEGMENT_RETURNDATA, 0, address, 32, pop_and_return_success)
    %jump(mstore_unpacking)

// On bad input, return empty return data but still return success.
ecrec_bad_input:
    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 0)
    %jump(pop_and_return_success)
