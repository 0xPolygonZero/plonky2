// Given a key, return a pointer to the associated SMT entry.
// Returns 0 if the key is not in the SMT.
global smt_read_state:
    // stack: key, retdest
    %split_key
    // stack: k0, k1, k2, k3, retdest
    PUSH 0
    // stack: level, k0, k1, k2, k3, retdest
    %mload_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT) // node_ptr
    // stack: node_ptr, level, k0, k1, k2, k3, retdest
    %jump(smt_read)

// Convenience macro to call smt_read_state and return where we left off.
%macro smt_read_state
    %stack (key) -> (key, %%after)
    %jump(smt_read_state)
%%after:
%endmacro

// Return a pointer to the data at the given key in the SMT at `trie_data[node_ptr]`.
// Pseudocode:
// ```
// read( HashNode { h }, key ) = if h == 0 then 0 else PANIC
// read( InternalNode { left, right }, key ) = if key&1 { read( right, key>>1 ) } else { read( left, key>>1 ) }
// read( Leaf { rem_key', value }, key ) = if rem_key == rem_key' then &value else 0
// ```
global smt_read:
    // stack: node_ptr, level, ks, retdest
    DUP1 %mload_trie_data
    // stack: node_type, node_ptr, level, ks, retdest
    // Increment node_ptr, so it points to the node payload instead of its type.
    SWAP1 %increment SWAP1
    // stack: node_type, node_payload_ptr, level, ks, retdest

    DUP1 %eq_const(@SMT_NODE_HASH)      %jumpi(smt_read_hash)
    DUP1 %eq_const(@SMT_NODE_INTERNAL)  %jumpi(smt_read_internal)
    DUP1 %eq_const(@SMT_NODE_LEAF)      %jumpi(smt_read_leaf)
    PANIC

smt_read_hash:
    // stack: node_type, node_payload_ptr, level, ks, retdest
    POP
    // stack: node_payload_ptr, level, ks, retdest
    %mload_trie_data
    // stack: hash, level, ks, retdest
    ISZERO %jumpi(smt_read_empty)
    PANIC // Trying to read a non-empty hash node. Should never happen.

smt_read_empty:
    %stack (level, k0, k1, k2, k3, retdest) -> (retdest, 0)
    JUMP

smt_read_internal:
    // stack: node_type, node_payload_ptr, level, ks, retdest
    POP
    // stack: node_payload_ptr, level, ks, retdest
    DUP2 %and_const(3) // level mod 4
    // stack: level%4, node_payload_ptr, level, ks, retdest
    DUP1 %eq_const(0) %jumpi(smt_read_internal_0)
    DUP1 %eq_const(1) %jumpi(smt_read_internal_1)
    DUP1 %eq_const(2) %jumpi(smt_read_internal_2)
    DUP1 %eq_const(3) %jumpi(smt_read_internal_3)
    PANIC
smt_read_internal_0:
    %stack (level_mod_4, node_payload_ptr, level, k0, k1, k2, k3 ) -> (k0, node_payload_ptr, level, k0, k1, k2, k3 )
    %pop_bit
    %stack (bit, newk0, node_payload_ptr, level, k0, k1, k2, k3 ) -> (bit, node_payload_ptr, level, newk0, k1, k2, k3 )
    %jump(smt_read_internal_contd)
smt_read_internal_1:
    %stack (level_mod_4, node_payload_ptr, level, k0, k1, k2, k3 ) -> (k1, node_payload_ptr, level, k0, k1, k2, k3 )
    %pop_bit
    %stack (bit, newk1, node_payload_ptr, level , k0, k1, k2, k3 ) -> (bit, node_payload_ptr, level, k0, newk1, k2, k3 )
    %jump(smt_read_internal_contd)
smt_read_internal_2:
    %stack (level_mod_4, node_payload_ptr, level, k0, k1, k2, k3 ) -> (k2, node_payload_ptr, level, k0, k1, k2, k3 )
    %pop_bit
    %stack (bit, newk2, node_payload_ptr, level, k0, k1, k2, k3 ) -> (bit, node_payload_ptr, level, k0, k1, newk2, k3 )
    %jump(smt_read_internal_contd)
smt_read_internal_3:
    %stack (level_mod_4, node_payload_ptr, level, k0, k1, k2, k3 ) -> (k3, node_payload_ptr, level, k0, k1, k2, k3 )
    %pop_bit
    %stack (bit, newk3, node_payload_ptr, level, k0, k1, k2, k3 ) -> (bit, node_payload_ptr, level, k0, k1, k2, newk3 )
smt_read_internal_contd:
    // stack: bit, node_payload_ptr, level, k0, k1, k2, k3, retdest
    ADD
    // stack: child_ptr_ptr, level, k0, k1, k2, k3, retdest
    %mload_trie_data
    // stack: child_ptr, level, k0, k1, k2, k3, retdest
    SWAP1 %increment SWAP1
    // stack: child_ptr, level+1, k0, k1, k2, k3, retdest
    %jump(smt_read)

smt_read_leaf:
    // stack: node_type, node_payload_ptr, level, ks, retdest
    POP
    // stack: node_payload_ptr, level, ks, retdest
    DUP1 %mload_trie_data
    // stack: rem_key, node_payload_ptr, level, ks, retdest
    SWAP1
    // stack: node_payload_ptr, rem_key, level, ks, retdest
    %increment
    %stack (value_ptr, rem_key, level, k0, k1, k2, k3) -> (k0, k1, k2, k3, rem_key, value_ptr)
    %combine_key
    // stack: this_rem_key, rem_key, value_ptr, retdest
    EQ %jumpi(smt_read_existing_leaf)
smt_read_non_existing_leaf:
    %stack (value_ptr, retdest) -> (retdest, 0)
    JUMP

smt_read_existing_leaf:
    // stack: value_ptr, retdest
    SWAP1 JUMP
