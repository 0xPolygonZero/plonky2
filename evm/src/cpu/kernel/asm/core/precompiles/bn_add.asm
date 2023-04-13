global precompile_bn_add:
    %stack (address, retdest, new_ctx, kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size) ->
        (new_ctx, kexit_info, ret_offset, ret_size)
    DUP1
    SET_CONTEXT
    // stack: (empty)
    PUSH 0x100000000 // = 2^32 (is_kernel = true)
    // stack: kexit_info

    PUSH @BN_ADD_GAS %charge_gas

    GET_CONTEXT
    %stack (ctx, kexit_info) -> (ctx, @SEGMENT_CALLDATA, 96, 32, bn_add_contd, kexit_info)
    %jump(mload_packing)
bn_add_contd:
    GET_CONTEXT
    %stack (ctx, y1, kexit_info) -> (ctx, @SEGMENT_CALLDATA, 64, 32, bn_add_contd2, y1, kexit_info)
    %jump(mload_packing)
bn_add_contd2:
    GET_CONTEXT
    %stack (ctx, x1, y1, kexit_info) -> (ctx, @SEGMENT_CALLDATA, 32, 32, bn_add_contd3, x1, y1, kexit_info)
    %jump(mload_packing)
bn_add_contd3:
    GET_CONTEXT
    %stack (ctx, y0, x1, y1, kexit_info) -> (ctx, @SEGMENT_CALLDATA, 0, 32, bn_add_contd4, y0, x1, y1, kexit_info)
    %jump(mload_packing)
bn_add_contd4:
    %stack (x0, y0, x1, y1, kexit_info) -> (x0, y0, x1, y1, bn_add_contd5, kexit_info)
    %jump(bn_add)
bn_add_contd5:
    // stack: x, y, kexit_info
    DUP2 %eq_const(@U256_MAX) // bn_add returns (U256_MAX, U256_MAX) on bad input.
    DUP2 %eq_const(@U256_MAX) // bn_add returns (U256_MAX, U256_MAX) on bad input.
    MUL // Cheaper than AND
    %jumpi(bn_add_bad_input)
    // stack: x, y, kexit_info

    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 64)
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    %stack (parent_ctx, x, y) -> (parent_ctx, @SEGMENT_RETURNDATA, 0, x, 32, bn_add_contd6, parent_ctx, y)
    %jump(mstore_unpacking)
bn_add_contd6:
    POP
    %stack (parent_ctx, y) -> (parent_ctx, @SEGMENT_RETURNDATA, 32, y, 32, pop_and_return_success)
    %jump(mstore_unpacking)

bn_add_bad_input:
    // stack: x, y, kexit_info
    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 0)
    POP
    %jump(pop_and_return_success)
