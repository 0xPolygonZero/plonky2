// Address where the working version of the hash value is stored.
%macro blake2_hash_value_addr
    PUSH 0
    // stack: 0
    %mload_current_general
    // stack: num_blocks
    %block_size
    %add_const(2)
    // stack: num_bytes+2
%endmacro

// Address where the working version of the compression internal state is stored.
%macro blake2_internal_state_addr
    %blake2_hash_value_addr
    %add_const(8)
%endmacro

// Address where the current message block is stored.
%macro blake2_message_addr
    %blake2_internal_state_addr
    %add_const(16)
%endmacro

// Block size is 128 bytes.
%macro block_size
    %mul_const(128)
%endmacro