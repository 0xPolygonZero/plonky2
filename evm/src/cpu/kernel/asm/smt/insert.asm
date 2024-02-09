// Insert a key-value pair in the state SMT.
global smt_insert_state:
    DUP2 ISZERO %jumpi(panic)
    // stack: key, value, retdest
    %stack (key, value) -> (key, value, smt_insert_state_after)
    %split_key
    // stack: k0, k1, k2, k3, value, smt_insert_state_after, retdest
    PUSH 0
    // stack: level, k0, k1, k2, k3, value, smt_insert_state_after, retdest
    %mload_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT) // node_ptr
    // stack: node_ptr, level, k0, k1, k2, k3, value, smt_insert_state_after, retdest
    %jump(smt_insert)

smt_insert_state_after:
    // stack: root_ptr, retdest
    %mstore_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT)
    // stack: retdest
    JUMP

%macro smt_insert_state
    %stack (key, value_ptr) -> (key, value_ptr, %%after)
    %jump(smt_insert_state)
%%after:
%endmacro

// Insert a key-value pair in the SMT at `trie_data[node_ptr]`.
// Pseudocode:
// ```
// insert( HashNode { h }, key, value ) = if h == 0 then Leaf { key, value } else PANIC
// insert( InternalNode { left, right }, key, value ) = if key&1 { insert( right, key>>1, value ) } else { insert( left, key>>1, value ) }
// insert( Leaf { key', value' }, key, value ) = {
//    let internal = new InternalNode;
//    insert(internal, key', value');
//    insert(internal, key, value);
//    return internal;}
// ```
global smt_insert:
    // stack: node_ptr, level, ks, value, retdest
    DUP1 %mload_trie_data
    // stack: node_type, node_ptr, level, ks, value, retdest
    // Increment node_ptr, so it points to the node payload instead of its type.
    SWAP1 %increment SWAP1
    // stack: node_type, node_payload_ptr, level, ks, value, retdest

    DUP1 %eq_const(@SMT_NODE_HASH)        %jumpi(smt_insert_hash)
    DUP1 %eq_const(@SMT_NODE_INTERNAL)    %jumpi(smt_insert_internal)
    DUP1 %eq_const(@SMT_NODE_LEAF)        %jumpi(smt_insert_leaf)
    PANIC

smt_insert_hash:
    // stack: node_type, node_payload_ptr, level, ks, value, retdest
    POP
    // stack: node_payload_ptr, level, ks, value, retdest
    %mload_trie_data
    // stack: hash, level, ks, value, retdest
    ISZERO %jumpi(smt_insert_empty)
    PANIC // Trying to insert in a non-empty hash node.
smt_insert_empty:
    // stack: level, ks, value, retdest
    POP
    // stack: ks, value, retdest
    %combine_key
    // stack: key, value, retdest
    %get_trie_data_size
    // stack: index, key, value, retdest
    PUSH @SMT_NODE_LEAF %append_to_trie_data
    %stack (index, key, value) -> (key, value, index)
    %append_to_trie_data // key
    %append_to_trie_data // value
    // stack: index, retdest
    SWAP1 JUMP

smt_insert_internal:
    // stack: node_type, node_payload_ptr, level, ks, value, retdest
    POP
    // stack: node_payload_ptr, level, ks, value, retdest
    DUP2 %and_const(3) // level mod 4
    // stack: level%4, node_payload_ptr, level, ks, value, retdest
    DUP1 %eq_const(0) %jumpi(smt_insert_internal_0)
    DUP1 %eq_const(1) %jumpi(smt_insert_internal_1)
    DUP1 %eq_const(2) %jumpi(smt_insert_internal_2)
    DUP1 %eq_const(3) %jumpi(smt_insert_internal_3)
    PANIC
smt_insert_internal_0:
    // stack: level%4, node_payload_ptr, level, ks, value, retdest
    %stack (level_mod_4, node_payload_ptr, level, k0, k1, k2, k3 ) -> (k0, node_payload_ptr, level, k0, k1, k2, k3 )
    %pop_bit
    %stack (bit, newk0, node_payload_ptr, level, k0, k1, k2, k3 ) -> (bit, node_payload_ptr, level, newk0, k1, k2, k3 )
    %jump(smt_insert_internal_contd)
smt_insert_internal_1:
    // stack: level%4, node_payload_ptr, level, ks, value, retdest
    %stack (level_mod_4, node_payload_ptr, level, k0, k1, k2, k3 ) -> (k1, node_payload_ptr, level, k0, k1, k2, k3 )
    %pop_bit
    %stack (bit, newk1, node_payload_ptr, level, k0, k1, k2, k3 ) -> (bit, node_payload_ptr, level, k0, newk1, k2, k3 )
    %jump(smt_insert_internal_contd)
smt_insert_internal_2:
    // stack: level%4, node_payload_ptr, level, ks, value, retdest
    %stack (level_mod_4, node_payload_ptr, level, k0, k1, k2, k3 ) -> (k2, node_payload_ptr, level, k0, k1, k2, k3 )
    %pop_bit
    %stack (bit, newk2, node_payload_ptr, level, k0, k1, k2, k3 ) -> (bit, node_payload_ptr, level, k0, k1, newk2, k3 )
    %jump(smt_insert_internal_contd)
smt_insert_internal_3:
    // stack: level%4, node_payload_ptr, level, ks, value, retdest
    %stack (level_mod_4, node_payload_ptr, level, k0, k1, k2, k3 ) -> (k3, node_payload_ptr, level, k0, k1, k2, k3 )
    %pop_bit
    %stack (bit, newk3, node_payload_ptr, level, k0, k1, k2, k3 ) -> (bit, node_payload_ptr, level, k0, k1, k2, newk3 )
    %jump(smt_insert_internal_contd)
smt_insert_internal_contd:
    // stack: bit, node_payload_ptr, level, ks, value, retdest
    DUP2 ADD
    // stack: child_ptr_ptr, node_payload_ptr, level, ks, value, retdest
    DUP1 %mload_trie_data
    // stack: child_ptr, child_ptr_ptr, node_payload_ptr, level, ks, value, retdest
    SWAP3 %increment SWAP3
    %stack (child_ptr, child_ptr_ptr, node_payload_ptr, level_plus_1, k0, k1, k2, k3, value, retdest) ->
            (child_ptr, level_plus_1, k0, k1, k2, k3, value, smt_insert_internal_after, child_ptr_ptr, node_payload_ptr, retdest)
    %jump(smt_insert)

smt_insert_internal_after:
    // stack: new_node_ptr, child_ptr_ptr, node_payload_ptr, retdest
    SWAP1 %mstore_trie_data
    // stack: node_payload_ptr, retdest
    %decrement
    SWAP1 JUMP

smt_insert_leaf:
    // stack: node_type, node_payload_ptr, level, ks, value, retdest
    POP
    %stack (node_payload_ptr, level, k0, k1, k2, k3, value) -> (k0, k1, k2, k3, node_payload_ptr, level, k0, k1, k2, k3, value)
    %combine_key
    // stack: rem_key, node_payload_ptr, level, ks, value, retdest
    DUP2 %mload_trie_data
    // stack: existing_key, rem_key, node_payload_ptr, level, ks, value, retdest
    DUP2 DUP2 EQ %jumpi(smt_insert_leaf_same_key)
    // stack: existing_key, rem_key, node_payload_ptr, level, ks, value, retdest
    // We create an internal node with two empty children, and then we insert the two leaves.
    %get_trie_data_size
    // stack: index, existing_key, rem_key, node_payload_ptr, level, ks, value, retdest
    PUSH @SMT_NODE_INTERNAL %append_to_trie_data
    PUSH 0 %append_to_trie_data // Empty hash node
    PUSH 0 %append_to_trie_data // Empty hash node
    // stack: index, existing_key, rem_key, node_payload_ptr, level, ks, value, retdest
    SWAP1 %split_key
    // stack: existing_k0, existing_k1, existing_k2, existing_k3, index, rem_key, node_payload_ptr, level, ks, value, retdest
    DUP7 %increment %mload_trie_data
    // stack: existing_value, existing_k0, existing_k1, existing_k2, existing_k3, index, rem_key, node_payload_ptr, level, ks, value, retdest
    DUP9
    %stack (level, existing_value, existing_k0, existing_k1, existing_k2, existing_k3, index) -> (index, level, existing_k0, existing_k1, existing_k2, existing_k3, existing_value, after_first_leaf)
    %jump(smt_insert)
after_first_leaf:
    // stack: internal_ptr, rem_key, node_payload_ptr, level, ks, value, retdest
    %stack (internal_ptr, rem_key, node_payload_ptr, level, k0, k1, k2, k3, value) -> (internal_ptr, level, k0, k1, k2, k3, value)
    %jump(smt_insert)

smt_insert_leaf_same_key:
    // stack: existing_key, rem_key, node_payload_ptr, level, ks, value, retdest
    %pop2
    %stack (node_payload_ptr, level, k0, k1, k2, k3, value) -> (node_payload_ptr, value, node_payload_ptr)
    %increment %mstore_trie_data
    // stack: node_payload_ptr, retdest
    %decrement
    // stack: node_ptr, retdest
    SWAP1 JUMP
