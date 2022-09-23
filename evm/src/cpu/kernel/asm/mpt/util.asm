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
    %add_const(1)
    // stack: trie_data_size', trie_data_size, value
    %set_trie_data_size
    // stack: trie_data_size, value
    %mstore_trie_data
    // stack: (empty)
%endmacro
