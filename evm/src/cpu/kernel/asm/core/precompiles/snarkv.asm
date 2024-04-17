global precompile_snarkv:
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

    PUSH 192 %calldatasize DUP2 DUP2
    // stack: calldata_size, 192, calldata_size, 192, kexit_info
    MOD %jumpi(fault_exception) // calldata_size should be a multiple of 192
    DIV
    // stack: k, kexit_info
    DUP1 %mul_const(@SNARKV_DYNAMIC_GAS) %add_const(@SNARKV_STATIC_GAS)
    %stack (gas, k, kexit_info) -> (gas, kexit_info, k)
    %charge_gas
    SWAP1
    // stack: k, kexit_info
    PUSH 0
loading_loop:
    // stack: i, k, kexit_info
    DUP2 DUP2 EQ %jumpi(loading_done)
    // stack: i, k, kexit_info
    DUP1 %mul_const(192)
    // stack: px, i, k, kexit_info
    GET_CONTEXT
    %stack (ctx, px) -> (ctx, @SEGMENT_CALLDATA, px, 32, loading_loop_contd, px)
    %build_address
    %jump(mload_packing)
loading_loop_contd:
    // stack: x, px, i, k, kexit_info
    SWAP1 %add_const(32)
    GET_CONTEXT
    %stack (ctx, py) -> (ctx, @SEGMENT_CALLDATA, py, 32, loading_loop_contd2, py)
    %build_address
    %jump(mload_packing)
loading_loop_contd2:
    // stack: y, py, x, i, k, kexit_info
    SWAP1 %add_const(32)
    GET_CONTEXT
    %stack (ctx, px_im) -> (ctx, @SEGMENT_CALLDATA, px_im, 32, loading_loop_contd3, px_im)
    %build_address
    %jump(mload_packing)
loading_loop_contd3:
    // stack: x_im, px_im, y, x, i, k, kexit_info
    SWAP1 %add_const(32)
    // stack: px_re, x_im, y, x, i, k, kexit_info
    GET_CONTEXT
    %stack (ctx, px_re) -> (ctx, @SEGMENT_CALLDATA, px_re, 32, loading_loop_contd4, px_re)
    %build_address
    %jump(mload_packing)
loading_loop_contd4:
    // stack: x_re, px_re, x_im, y, x, i, k, kexit_info
    SWAP1 %add_const(32)
    // stack: py_im, x_re, x_im, y, x, i, k, kexit_info
    GET_CONTEXT
    %stack (ctx, py_im) -> (ctx, @SEGMENT_CALLDATA, py_im, 32, loading_loop_contd5, py_im)
    %build_address
    %jump(mload_packing)
loading_loop_contd5:
    // stack: y_im, py_im, x_re, x_im, y, x, i, k, kexit_info
    SWAP1 %add_const(32)
    // stack: py_re, y_im, x_re, x_im, y, x, i, k, kexit_info
    GET_CONTEXT
    %stack (ctx, py_re) -> (ctx, @SEGMENT_CALLDATA, py_re, 32, loading_loop_contd6)
    %build_address
    %jump(mload_packing)
loading_loop_contd6:
    // stack: y_re, y_im, x_re, x_im, y, x, i, k, kexit_info
    SWAP1  // the EVM serializes the imaginary part first
    // stack: y_im, y_re, x_re, x_im, y, x, i, k, kexit_info
    DUP7
    // stack: i, y_im, y_re, x_re, x_im, y, x, i, k, kexit_info
    %mul_const(6) %add_const(@SNARKV_INP)
    %add_const(5)
    %mstore_bn254_pairing
    // stack: y_re, x_re, x_im, y, x, i, k, kexit_info
    DUP6
    // stack: i, y_re, x_re, x_im, y, x, i, k, kexit_info
    %mul_const(6) %add_const(@SNARKV_INP)
    %add_const(4)
    %mstore_bn254_pairing
    SWAP1  // the EVM serializes the imaginary part first
    // stack: x_im, x_re, y, x, i, k, kexit_info
    DUP5
    // stack: i, x_im, x_re, y, x, i, k, kexit_info
    %mul_const(6) %add_const(@SNARKV_INP)
    %add_const(3)
    %mstore_bn254_pairing
    // stack: x_re, y, x, i, k, kexit_info
    DUP4
    // stack: i, x_re, y, x, i, k, kexit_info
    %mul_const(6) %add_const(@SNARKV_INP)
    %add_const(2)
    %mstore_bn254_pairing
    // stack: y, x, i, k, kexit_info
    DUP3
    // stack: i, y, x, i, k, kexit_info
    %mul_const(6) %add_const(@SNARKV_INP)
    %add_const(1)
    %mstore_bn254_pairing
    // stack: x, i, k, kexit_info
    DUP2
    // stack: i, x, i, k, kexit_info
    %mul_const(6) %add_const(@SNARKV_INP)
    %mstore_bn254_pairing
    // stack: i, k, kexit_info
    %increment
    %jump(loading_loop)

loading_done:
    %stack (i, k) -> (k, @SNARKV_INP, @SNARKV_OUT, got_result)
    %jump(bn254_pairing)
got_result:
    // stack: result, kexit_info
    DUP1 %eq_const(@U256_MAX) %jumpi(fault_exception)
    // stack: result, kexit_info
    // Store the result bool (repr. by a U256) to the parent's return data using `mstore_unpacking`.
    %mstore_parent_context_metadata(@CTX_METADATA_RETURNDATA_SIZE, 32)
    %mload_context_metadata(@CTX_METADATA_PARENT_CONTEXT)
    %stack (parent_ctx, address) -> (parent_ctx, @SEGMENT_RETURNDATA, address, 32, pop_and_return_success)
    %build_address_no_offset
    %jump(mstore_unpacking)
