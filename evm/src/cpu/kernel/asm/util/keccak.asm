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
