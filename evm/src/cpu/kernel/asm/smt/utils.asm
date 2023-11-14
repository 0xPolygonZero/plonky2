%macro pop_bit
    // stack: key
    DUP1 %shr_const(1)
    // stack: key>>1, key
    SWAP1 %and_const(1)
    // stack: key&1, key>>1
%endmacro

// Returns a non-zero value if the node is non-empty.
%macro is_non_empty_node
    // stack: node_ptr
    DUP1 %mload_trie_data %jumpi(%%end) // If the node is not a hash node, node_ptr is non-zero.
    // The node is a hash node
    // stack: node_ptr
    %increment %mload_trie_data
    // stack: hash
    %jump(%%end)
%%end:
%endmacro
