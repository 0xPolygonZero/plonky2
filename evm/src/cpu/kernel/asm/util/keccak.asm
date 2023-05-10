global sys_keccak256:
    // stack: kexit_info, offset, len
    PUSH @GAS_KECCAK256
    DUP4
    // stack: len, static_gas, kexit_info, offset, len
    ISZERO %jumpi(sys_keccak256_empty)
    // stack: static_gas, kexit_info, offset, len
    DUP4 %num_bytes_to_num_words %mul_const(@GAS_KECCAK256WORD)
    ADD
    %charge_gas
    // stack: kexit_info, offset, len

    %stack (kexit_info, offset, len) -> (offset, len, kexit_info, offset, len)
    %add_or_fault
    DUP1 %ensure_reasonable_offset
    %update_mem_bytes

    %stack (kexit_info, offset, len) -> (offset, len, kexit_info)
    PUSH @SEGMENT_MAIN_MEMORY
    GET_CONTEXT
    // stack: ADDR: 3, len, kexit_info
    KECCAK_GENERAL
    // stack: hash, kexit_info
    SWAP1
    EXIT_KERNEL

sys_keccak256_empty:
    // stack: static_gas, kexit_info, offset, len
    %charge_gas
    %stack (kexit_info, offset, len) -> (kexit_info, @EMPTY_STRING_HASH)
    EXIT_KERNEL

// Computes Keccak256(input_word). Clobbers @SEGMENT_KERNEL_GENERAL.
//
// Pre stack: input_word
// Post stack: hash
%macro keccak256_word(num_bytes)
    // Since KECCAK_GENERAL takes its input from memory, we will first write
    // input_word's bytes to @SEGMENT_KERNEL_GENERAL[0..$num_bytes].
    %stack (word) -> (0, @SEGMENT_KERNEL_GENERAL, 0, word, $num_bytes, %%after_mstore)
    %jump(mstore_unpacking)
%%after_mstore:
    // stack: offset
    %stack (offset) -> (0, @SEGMENT_KERNEL_GENERAL, 0, $num_bytes) // context, segment, offset, len
    KECCAK_GENERAL
%endmacro

// Computes Keccak256(a || b). Clobbers @SEGMENT_KERNEL_GENERAL.
//
// Pre stack: a, b
// Post stack: hash
%macro keccak256_u256_pair
    // Since KECCAK_GENERAL takes its input from memory, we will first write
    // a's bytes to @SEGMENT_KERNEL_GENERAL[0..32], then b's bytes to
    // @SEGMENT_KERNEL_GENERAL[32..64].
    %stack (a) -> (0, @SEGMENT_KERNEL_GENERAL, 0, a, 32, %%after_mstore_a)
    %jump(mstore_unpacking)
%%after_mstore_a:
    %stack (offset, b) -> (0, @SEGMENT_KERNEL_GENERAL, 32, b, 32, %%after_mstore_b)
    %jump(mstore_unpacking)
%%after_mstore_b:
    %stack (offset) -> (0, @SEGMENT_KERNEL_GENERAL, 0, 64) // context, segment, offset, len
    KECCAK_GENERAL
%endmacro
