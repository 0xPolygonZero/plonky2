// Generate a new hash value from the previous hash value and two elements of the internal state.
blake2_generate_new_hash_value:
    // stack: i, retdest
    %blake2_hash_value_addr
    // stack: addr, i, retdest
    DUP2
    ADD
    %mload_current_general
    // stack: h_i, i, retdest
    %blake2_internal_state_addr
    // stack: addr, h_i, i, retdest
    DUP3
    ADD
    %mload_current_general
    // stack: v_i, h_i, i, retdest
    %blake2_internal_state_addr
    // stack: addr, v_i, h_i, i, retdest
    SWAP1
    // stack: v_i, addr, h_i, i, retdest
    SWAP3
    // stack: i, addr, h_i, v_i, retdest
    ADD
    %add_const(8)
    %mload_current_general
    // stack: v_(i+8), h_i, v_i, retdest
    XOR
    XOR
    // stack: h_i' = v_(i+8) ^ v_i ^ h_i, retdest
    SWAP1
    JUMP

global blake2_generate_all_hash_values:
    // stack: retdest
    PUSH 8
    // stack: i=8, retdest
blake2_generate_hash_loop:
    // stack: i, h_i', ..., h_7', retdest
    %decrement
    // stack: i-1, h_i', ..., h_7', retdest
    PUSH blake2_generate_hash_return
    // stack: blake2_generate_hash_return, i-1, h_i', ..., h_7', retdest
    DUP2
    // stack: i-1, blake2_generate_hash_return, i-1, h_i', ..., h_7', retdest
    %jump(blake2_generate_new_hash_value)
blake2_generate_hash_return:
    // stack: h_(i-1)', i-1, h_i', ..., h_7', retdest
    SWAP1
    // stack: i-1, h_(i-1)', h_i', ..., h_7', retdest
    DUP1
    // stack: i-1, i-1, h_(i-1)', ..., h_7', retdest
    %jumpi(blake2_generate_hash_loop)
    // stack: i-1=0, h_0', ..., h_7', retdest
    %stack (i, h: 8, ret) -> (ret, h)
    // stack: retdest, h_0'...h_7'
    JUMP
