global precompile_snarkv:
    // stack: address, retdest, new_ctx, (old stack)
    %pop2
    // stack: new_ctx, (old stack)
    DUP1
    SET_CONTEXT
    // stack: (empty)
    PUSH 0x100000000 // = 2^32 (is_kernel = true)
    // stack: kexit_info

    PUSH 192 %calldatasize DUP2 DUP2
    // stack: calldata_size, 192, calldata_size, 192, kexit_info
    MOD %jumpi(fault_exception) // calldata_size should be a multiple of 192
    DIV
    // stack: k, kexit_info
    DUP1 %mul_const(@SNARKV_DYNAMIC_GAS) @add_const(@SNARKV_STATIC_GAS)
    %stack (gas, k, kexit_info) -> (gas, kexit_info, k)
    %charge_gas
    SWAP1
    // stack: k, kexit_info
    PUSH 0
loading_loop:
    // stack: i, k, kexit_info
    DUP2 DUP2 EQ %jumpi(loading_done)
    // stack: i, k, kexit_info
    // stack: i, k, kexit_info
    DUP1 %mul_const(192)
    // stack: px, i, k, kexit_info
    GET_CONTEXT
    %stack (ctx, px) -> (ctx, @SEGMENT_CALLDATA, px, 32, loading_loop_contd, px)
    %jump(mload_packing)
loading_loop_contd:
    // stack: x, px, i, k, kexit_info
    SWAP1 %add_const(32)
    %stack (py) -> (ctx, @SEGMENT_CALLDATA, py, 32, loading_loop_contd2, py)
    %jump(mload_packing)
loading_loop_contd2:
    // stack: y, py, x, i, k, kexit_info
    SWAP1 %add_const(32)
    %stack (px_re) -> (ctx, @SEGMENT_CALLDATA, px_re, 32, loading_loop_contd3, px_re)
    %jump(mload_packing)
loading_loop_contd3:
    // stack: x_re, px_re, y, x, i, k, kexit_info
    SWAP1 %add_const(32)
    // stack: px_im, x_re, y, x, i, k, kexit_info
    %stack (px_im) -> (ctx, @SEGMENT_CALLDATA, px_im, 32, loading_loop_contd4, px_im)
    %jump(mload_packing)
loading_loop_contd4:
    // stack: x_im, px_im, x_re, y, x, i, k, kexit_info
    SWAP1 %add_const(32)
    // stack: py_re, x_im, x_re, y, x, i, k, kexit_info
    %stack (py_re) -> (ctx, @SEGMENT_CALLDATA, py_re, 32, loading_loop_contd5, py_re)
    %jump(mload_packing)
loading_loop_contd5:
    // stack: y_re, py_re, x_im, x_re, y, x, i, k, kexit_info
    SWAP1 %add_const(32)
    // stack: py_im, y_re, x_im, x_re, y, x, i, k, kexit_info
    %stack (py_im) -> (ctx, @SEGMENT_CALLDATA, py_im, 32, loading_loop_contd6)
    %jump(mload_packing)
loading_loop_contd6:
    // stack: y_im, y_re, x_im, x_re, y, x, i, k, kexit_info
    DUP7
    // stack: i, y_im, y_re, x_im, x_re, y, x, i, k, kexit_info
    %mul_const(6) %add_const(@SNARKV_INP)
    %add_const(5)
    %mstore_kernel_bn254_pairing
    // stack: y_re, x_im, x_re, y, x, i, k, kexit_info
    DUP6
    // stack: i, y_re, x_im, x_re, y, x, i, k, kexit_info
    %mul_const(6) %add_const(@SNARKV_INP)
    %add_const(4)
    %mstore_kernel_bn254_pairing
    // stack: x_im, x_re, y, x, i, k, kexit_info
    DUP5
    // stack: i, x_im, x_re, y, x, i, k, kexit_info
    %mul_const(6) %add_const(@SNARKV_INP)
    %add_const(3)
    %mstore_kernel_bn254_pairing
    // stack: x_re, y, x, i, k, kexit_info
    DUP4
    // stack: i, x_re, y, x, i, k, kexit_info
    %mul_const(6) %add_const(@SNARKV_INP)
    %add_const(2)
    %mstore_kernel_bn254_pairing
    // stack: y, x, i, k, kexit_info
    DUP3
    // stack: i, y, x, i, k, kexit_info
    %mul_const(6) %add_const(@SNARKV_INP)
    %add_const(1)
    %mstore_kernel_bn254_pairing
    // stack: x, i, k, kexit_info
    DUP2
    // stack: i, x, i, k, kexit_info
    %mul_const(6) %add_const(@SNARKV_INP)
    %mstore_kernel_bn254_pairing
    // stack: i, k, kexit_info
    %increment
    %jump(loading_loop)

loading_done:
    // stack: i, k, kexit_info
    %pop2
