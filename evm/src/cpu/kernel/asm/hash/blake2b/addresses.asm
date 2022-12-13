// Load the initial hash value (the IV, but with params XOR'd into the first word).
%macro blake2b_initial_hash_value
    %blake2b_iv_i(7)
    %blake2b_iv_i(6)
    %blake2b_iv_i(5)
    %blake2b_iv_i(4)
    %blake2b_iv_i(3)
    %blake2b_iv_i(2)
    %blake2b_iv_i(1)
    // stack: IV_1, IV_2, IV_3, IV_4, IV_5, IV_6, IV_7
    PUSH 0x01010040 // params: key = 00, digest_size = 64 = 0x40
    %blake2b_iv_i(0)
    XOR
    // stack: IV_0 ^ params, IV_1, IV_2, IV_3, IV_4, IV_5, IV_6, IV_7
%endmacro

// Address where the working version of the hash value is stored.
%macro blake2b_hash_value_addr
    PUSH 0
    // stack: 0
    %mload_kernel_general
    // stack: num_blocks
    %block_size
    %add_const(2)
    // stack: num_bytes+2
%endmacro

// Address where the working version of the compression internal state is stored.
%macro blake2b_internal_state_addr
    %blake2b_hash_value_addr
    %add_const(8)
%endmacro

// Address where the current message block is stored.
%macro blake2b_message_addr
    %blake2b_internal_state_addr
    %add_const(16)
%endmacro

// Block size is 128 bytes.
%macro block_size
    %mul_const(128)
%endmacro