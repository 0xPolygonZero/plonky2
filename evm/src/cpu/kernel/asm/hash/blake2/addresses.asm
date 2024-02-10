// Address where the working version of the hash value is stored.
// It is ready to be used, i.e. already containing the current context
// and SEGMENT_KERNEL_GENERAL.
%macro blake2_hash_value_addr
    %build_current_general_address_no_offset
    DUP1
    MLOAD_GENERAL
    // stack: num_blocks, addr
    %block_size
    %add_const(2)
    // stack: num_bytes+2, addr
    ADD
    // stack: addr
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