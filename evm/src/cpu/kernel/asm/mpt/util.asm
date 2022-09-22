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
