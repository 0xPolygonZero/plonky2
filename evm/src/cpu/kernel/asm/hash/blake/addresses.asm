// Load the initial hash value (the IV, but with params XOR'd into the first word).
%macro blake_initial_hash_value
    %blake_iv_i(7)
    %blake_iv_i(6)
    %blake_iv_i(5)
    %blake_iv_i(4)
    %blake_iv_i(3)
    %blake_iv_i(2)
    %blake_iv_i(1)
    // stack: IV_1, IV_2, IV_3, IV_4, IV_5, IV_6, IV_7
    PUSH 0x01010040 // params: key = 00, digest_size = 64 = 0x40
    %blake_iv_i(0)
    XOR
    // stack: IV_0 ^ params, IV_1, IV_2, IV_3, IV_4, IV_5, IV_6, IV_7
%endmacro

// Address where the working version of the hash value is stored.
%macro blake_hash_value_addr
    PUSH 0
    // stack: 0
    %mload_kernel_general
    // stack: num_blocks
    %mul_const(128)
    %add_const(2)
    // stack: num_bytes+2
%endmacro

// Address where the working version of the compression internal state is stored.
%macro blake_internal_state_addr
    %blake_hash_value_addr
    %add_const(8)
%endmacro

// Address where the current message block is stored.
%macro blake_message_addr
    %blake_internal_state_addr
    %add_const(16)
%endmacro
