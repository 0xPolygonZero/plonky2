blake2b_generate_new_hash_value:
    // stack: i, retdest
    %blake2b_hash_value_addr
    // stack: addr, i, retdest
    DUP2
    ADD
    %mload_kernel_general
    // stack: h_i, i, retdest
    %blake2b_internal_state_addr
    // stack: addr, h_i, i, retdest
    DUP3
    ADD
    %mload_kernel_general
    // stack: v_i, h_i, i, retdest
    %blake2b_internal_state_addr
    // stack: addr, v_i, h_i, i, retdest
    DUP4
    ADD
    %add_const(8)
    %mload_kernel_general
    // stack: v_(i+8), v_i, h_i, i, retdest
    XOR
    XOR
    // stack: h_i' = v_(i+8) ^ v_i ^ h_i, i, retdest
    SWAP1
    POP
    // stack: h_i', retdest
    SWAP1
    JUMP

global blake2b_generate_all_hash_values:
    // stack: retdest
    PUSH blake2b_generate_hash_return_7
    // stack: blake2b_generate_hash_return_7, retdest
    PUSH 7
    // stack: 7, blake2b_generate_hash_return_7, retdest
    %jump(blake2b_generate_new_hash_value)
blake2b_generate_hash_return_7:
    // stack: h_7', retdest
    PUSH blake2b_generate_hash_return_6
    // stack: blake2b_generate_hash_return_6, h_7', retdest
    PUSH 6
    // stack: 6, blake2b_generate_hash_return_6, h_7', retdest
    %jump(blake2b_generate_new_hash_value)
blake2b_generate_hash_return_6:
    // stack: h_6', h_7', retdest
    PUSH blake2b_generate_hash_return_5
    // stack: blake2b_generate_hash_return_5, h_6', h_7', retdest
    PUSH 5
    // stack: 5, blake2b_generate_hash_return_5, h_6', h_7', retdest
    %jump(blake2b_generate_new_hash_value)
blake2b_generate_hash_return_5:
    // stack: h_5', h_6', h_7', retdest
    PUSH blake2b_generate_hash_return_4
    // stack: blake2b_generate_hash_return_4, h_5', h_6', h_7', retdest
    PUSH 4
    // stack: 4, blake2b_generate_hash_return_4, h_5', h_6', h_7', retdest
    %jump(blake2b_generate_new_hash_value)
blake2b_generate_hash_return_4:
    // stack: h_4', h_5', h_6', h_7', retdest
    PUSH blake2b_generate_hash_return_3
    // stack: blake2b_generate_hash_return_3, h_4', h_5', h_6', h_7', retdest
    PUSH 3
    // stack: 3, blake2b_generate_hash_return_3, h_4', h_5', h_6', h_7', retdest
    %jump(blake2b_generate_new_hash_value)
blake2b_generate_hash_return_3:
    // stack: h_3', h_4', h_5', h_6', h_7', retdest
    PUSH blake2b_generate_hash_return_2
    // stack: blake2b_generate_hash_return_2, h_3', h_4', h_5', h_6', h_7', retdest
    PUSH 2
    // stack: 2, blake2b_generate_hash_return_2, h_3', h_4', h_5', h_6', h_7', retdest
    %jump(blake2b_generate_new_hash_value)
blake2b_generate_hash_return_2:
    // stack: h_2', h_3', h_4', h_5', h_6', h_7', retdest
    PUSH blake2b_generate_hash_return_1
    // stack: blake2b_generate_hash_return_1, h_2', h_3', h_4', h_5', h_6', h_7', retdest
    PUSH 1
    // stack: 1, blake2b_generate_hash_return_1, h_2', h_3', h_4', h_5', h_6', h_7', retdest
    %jump(blake2b_generate_new_hash_value)
blake2b_generate_hash_return_1:
    // stack: h_1', h_2', h_3', h_4', h_5', h_6', h_7', retdest
    PUSH blake2b_generate_hash_return_0
    // stack: blake2b_generate_hash_return_0, h_1', h_2', h_3', h_4', h_5', h_6', h_7', retdest
    PUSH 0
    // stack: 0, blake2b_generate_hash_return_0, h_1', h_2', h_3', h_4', h_5', h_6', h_7', retdest
    %jump(blake2b_generate_new_hash_value)
blake2b_generate_hash_return_0:
    // stack: h_0', h_1', h_2', h_3', h_4', h_5', h_6', h_7', retdest
    %stack (h: 8, ret) -> (ret, h)
    // stack: retdest, h_0', h_1', h_2', h_3', h_4', h_5', h_6', h_7'
    JUMP