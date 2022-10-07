%macro mload_trie_data
    // stack: virtual
    %mload_kernel(@SEGMENT_TRIE_DATA)
    // stack: value
%endmacro

%macro mstore_trie_data
    // stack: virtual, value
    %mstore_kernel(@SEGMENT_TRIE_DATA)
    // stack: (empty)
%endmacro

%macro get_trie_data_size
    // stack: (empty)
    %mload_global_metadata(@GLOBAL_METADATA_TRIE_DATA_SIZE)
    // stack: trie_data_size
%endmacro

%macro set_trie_data_size
    // stack: trie_data_size
    %mstore_global_metadata(@GLOBAL_METADATA_TRIE_DATA_SIZE)
    // stack: (empty)
%endmacro

// Equivalent to: trie_data[trie_data_size++] = value
%macro append_to_trie_data
    // stack: value
    %get_trie_data_size
    // stack: trie_data_size, value
    DUP1
    %increment
    // stack: trie_data_size', trie_data_size, value
    %set_trie_data_size
    // stack: trie_data_size, value
    %mstore_trie_data
    // stack: (empty)
%endmacro

// Split off the first nibble from a key part. Roughly equivalent to
// def split_first_nibble(num_nibbles, key):
//     num_nibbles -= 1
//     num_nibbles_x4 = num_nibbles * 4
//     first_nibble = (key >> num_nibbles_x4) & 0xF
//     key -= (first_nibble << num_nibbles_x4)
//     return (first_nibble, num_nibbles, key)
%macro split_first_nibble
    // stack: num_nibbles, key
    %decrement // num_nibbles -= 1
    // stack: num_nibbles, key
    DUP2
    // stack: key, num_nibbles, key
    DUP2 %mul_const(4)
    // stack: num_nibbles_x4, key, num_nibbles, key
    SHR
    // stack: key >> num_nibbles_x4, num_nibbles, key
    %and_const(0xF)
    // stack: first_nibble, num_nibbles, key
    DUP1
    // stack: first_nibble, first_nibble, num_nibbles, key
    DUP3 %mul_const(4)
    // stack: num_nibbles_x4, first_nibble, first_nibble, num_nibbles, key
    SHL
    // stack: first_nibble << num_nibbles_x4, first_nibble, num_nibbles, key
    DUP1
    // stack: junk, first_nibble << num_nibbles_x4, first_nibble, num_nibbles, key
    SWAP4
    // stack: key, first_nibble << num_nibbles_x4, first_nibble, num_nibbles, junk
    SUB
    // stack: key, first_nibble, num_nibbles, junk
    SWAP3
    // stack: junk, first_nibble, num_nibbles, key
    POP
    // stack: first_nibble, num_nibbles, key
%endmacro
