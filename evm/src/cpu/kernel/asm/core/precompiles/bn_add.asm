global precompile_bn_add:
    // stack: address, retdest, new_ctx, (old stack)
    %pop2
    // stack: new_ctx, (old stack)
    DUP1
    SET_CONTEXT
    // stack: (empty)
    PUSH 0x100000000 // = 2^32 (is_kernel = true)
    // stack: kexit_info

    %charge_gas_const(@BN_ADD_GAS)

    // Load x0, y0, x1, y1 from the call data using `mload_packing`.
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
    %jumpi(fault_exception)
    // stack: x, y, kexit_info

    // Store the result (x, y) to the parent's return data using `mstore_unpacking`.
    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 64)
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    %stack (parent_ctx, x, y) -> (parent_ctx, @SEGMENT_RETURNDATA, 0, x, 32, bn_add_contd6, parent_ctx, y)
    %jump(mstore_unpacking)
bn_add_contd6:
    POP
    %stack (parent_ctx, y) -> (parent_ctx, @SEGMENT_RETURNDATA, 32, y, 32, pop_and_return_success)
    %jump(mstore_unpacking)
