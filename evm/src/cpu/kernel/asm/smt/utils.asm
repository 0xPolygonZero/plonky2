// Input: x
// Output: (x&1, x>>1)
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

// Input: key = k0 + k1.2^64 + k2.2^128 + k3.2^192, with 0<=ki<2^64.
// Output: (k0, k1, k2, k3)
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

// Input: (k0, k1, k2, k3)
// Output: k0 + k1.2^64 + k2.2^128 + k3.2^192
%macro combine_key
    // stack: k0, k1, k2, k3
    SWAP1 %shl_const(64) ADD
    // stack: k0 + k1<<64, k2, k3
    SWAP1 %shl_const(128) ADD
    // stack: k0 + k1<<64 + k2<<128, k3
    SWAP1 %shl_const(192) ADD
    // stack: k0 + k1<<64 + k2<<128 + k3<<192
%endmacro


// Pseudocode:
// ```
// def recombine_key(key, bit, level):
//   obit = 1-bit
//   k0, k1, k2, k3 = [(key>>(64*i))&(2**64-1) for i in range(4)]
//   match level%4:
//     0 => k0 = 2*k0 + obit
//     1 => k1 = 2*k1 + obit
//     2 => k2 = 2*k2 + obit
//     3 => k3 = 2*k3 + obit
//   return k0 + (k1<<64) + (k2<<128) + (k3<<192)
// ```
%macro recombine_key
    // stack: key, bit, level
    SWAP1 PUSH 1 SUB
    // stack: obit, key, level
    SWAP2
    // stack: level, key, obit
    %and_const(3)
    // stack: level%4, key, obit
    DUP1 %eq_const(0) %jumpi(%%recombine_key_0)
    DUP1 %eq_const(1) %jumpi(%%recombine_key_1)
    DUP1 %eq_const(2) %jumpi(%%recombine_key_2)
    DUP1 %eq_const(3) %jumpi(%%recombine_key_3)
    PANIC
%%recombine_key_0:
    // stack: level%4, key, obit
    POP
    // stack: key, obit
    %split_key
    // stack: k0, k1, k2, k3, obit
    %shl_const(1)
    // stack: k0<<1, k1, k2, k3, obit
    DUP5 ADD
    // stack: k0<<1 + obit, k1, k2, k3, obit
    %combine_key
    %stack (newkey, obit) -> (newkey)
    %jump(%%after)
%%recombine_key_1:
    // stack: level%4, key, obit
    POP
    // stack: key, obit
    %split_key
    // stack: k0, k1, k2, k3, obit
    DUP2 %shl_const(1)
    // stack: k1<<1, k0, k1, k2, k3, obit
    DUP6 ADD
    // stack: k1<<1 + obit, k0, k1, k2, k3, obit
    SWAP2 POP
    %combine_key
    %stack (newkey, obit) -> (newkey)
    %jump(%%after)
%%recombine_key_2:
    // stack: key, obit
    POP
    // stack: key, obit
    %split_key
    // stack: k0, k1, k2, k3, obit
    DUP3 %shl_const(1)
    // stack: k2<<1, k0, k1, k2, k3, obit
    DUP6 ADD
    // stack: k2<<1 + obit, k0, k1, k2, k3, obit
    SWAP3 POP
    %combine_key
    %stack (newkey, obit) -> (newkey)
    %jump(%%after)
%%recombine_key_3:
    // stack: key, obit
    POP
    // stack: key, obit
    %split_key
    // stack: k0, k1, k2, k3, obit
    DUP4 %shl_const(1)
    // stack: k3<<1, k0, k1, k2, k3, obit
    DUP6 ADD
    // stack: k3<<1 + obit, k0, k1, k2, k3, obit
    SWAP4 POP
    %combine_key
    %stack (newkey, obit) -> (newkey)
%%after:
    // stack: newkey
%endmacro
