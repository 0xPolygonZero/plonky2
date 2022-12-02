%macro blake2b_generate_new_hash_value(i)
    %blake2b_hash_value_addr
    %add_const($i)
    %mload_kernel_general
    // stack: h_i, ...
    %blake2b_internal_state_addr
    %add_const($i)
    %mload_kernel_general
    // stack: v_i, h_i, ...
    %blake2b_internal_state_addr
    %add_const($i)
    %add_const(8)
    %mload_kernel_general
    // stack: v_(i+8), v_i, h_i, ...
    XOR
    XOR
    // stack: h_i' = v_(i+8) ^ v_i ^ h_i, ...
%endmacro
