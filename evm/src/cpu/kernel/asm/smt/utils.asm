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

%macro split_key
    // stack: key
    DUP1 %shr_const(128) %and_const(0xffffffffffffffff)
    // stack: k2, key
    DUP2 %shr_const(64) %and_const(0xffffffffffffffff)
    // stack: k1, k2, key
    DUP3 %shr_const(192)
    // stack: k3, k1, k2, key
    SWAP3 %and_const(0xffffffffffffffff)
    // stack: k0, k1, k2, k3
%endmacro

%macro combine_key
    // stack: k0, k1, k2, k3
    SWAP1 %shl_const(64) ADD
    // stack: k0 + k1<<64, k2, k3
    SWAP1 %shl_const(128) ADD
    // stack: k0 + k1<<64 + k2<<128, k3
    SWAP1 %shl_const(192) ADD
    // stack: k0 + k1<<64 + k2<<128 + k3<<192
%endmacro
