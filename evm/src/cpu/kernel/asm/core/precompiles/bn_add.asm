global precompile_bn_add:
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

    %charge_gas_const(@BN_ADD_GAS)

    // Load x0, y0, x1, y1 from the call data using `mload_packing`.
    PUSH bn_add_return
    // stack: bn_add_return, kexit_info
    %stack () -> (@SEGMENT_CALLDATA, 96, 32)
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 96, 32, bn_add_return, kexit_info
    %build_address
    %mload_packing
    // stack: y1, bn_add_return, kexit_info
    %stack () -> (@SEGMENT_CALLDATA, 64, 32)
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 64, 32, y1, bn_add_return, kexit_info
    %build_address
    %mload_packing
    // stack: x1, y1, bn_add_return, kexit_info
    %stack () -> (@SEGMENT_CALLDATA, 32, 32)
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 32, 32, x1, y1, bn_add_return, kexit_info
    %build_address
    %mload_packing
    // stack: y0, x1, y1, bn_add_return, kexit_info
    %stack () -> (@SEGMENT_CALLDATA, 32)
    GET_CONTEXT
    // stack: ctx, @SEGMENT_CALLDATA, 32, y0, x1, y1, bn_add_return, kexit_info
    %build_address_no_offset
    %mload_packing
    // stack: x0, y0, x1, y1, bn_add_return, kexit_info
    %jump(bn_add)
bn_add_return:
    // stack: x, y, kexit_info
    DUP2 %eq_const(@U256_MAX) // bn_add returns (U256_MAX, U256_MAX) on bad input.
    DUP2 %eq_const(@U256_MAX) // bn_add returns (U256_MAX, U256_MAX) on bad input.
    MUL // Cheaper than AND
    %jumpi(fault_exception)
    // stack: x, y, kexit_info

    // Store the result (x, y) to the parent's return data using `mstore_unpacking`.
    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 64)
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    %stack (parent_ctx, x, y) -> (parent_ctx, @SEGMENT_RETURNDATA, x, 32, bn_add_contd6, parent_ctx, y)
    %build_address_no_offset
    %jump(mstore_unpacking)
bn_add_contd6:
    POP
    %stack (parent_ctx, y) -> (parent_ctx, @SEGMENT_RETURNDATA, 32, y, 32, pop_and_return_success)
    %build_address
    %jump(mstore_unpacking)
