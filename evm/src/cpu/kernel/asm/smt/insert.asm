// Insert a key-value pair in the state SMT.
global smt_insert_state:
    // stack: key, new_account_ptr, retdest
    %stack (key, new_account_ptr) -> (key, new_account_ptr, smt_insert_state_set_root)
    %mload_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT)
    // stack: root_ptr, key, new_account_ptr, smt_insert_state_set_root, retdest
    %jump(smt_insert)

smt_insert_state_set_root:
    // stack: root_ptr, retdest
    %mstore_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT)
    // stack: retdest
    JUMP

// Insert a key-value pair in the SMT at `trie_data[node_ptr]`.
// `value_ptr`  should point to a an empty slot reserved for `rem_key`, followed by the actual value.
// Pseudocode:
// ```
// insert( HashNode { h }, key, value_ptr ) = if h == 0 then Leaf { key, value_ptr } else PANIC
// insert( InternalNode { left, right }, key, value_ptr ) = if key&1 { insert( right, key>>1, value_ptr ) } else { insert( left, key>>1, value_ptr ) }
// insert( Leaf { key', value_ptr' }, key, value_ptr ) = {
//    let internal = new InternalNode;
//    insert(internal, key', value_ptr');
//    insert(internal, key, value_ptr);
//    return internal;}
// ```
global smt_insert:
    // stack: node_ptr, key, value_ptr, retdest
    DUP1 %mload_trie_data
    // stack: node_type, node_ptr, key, value_ptr, retdest
    // Increment node_ptr, so it points to the node payload instead of its type.
    SWAP1 %increment SWAP1
    // stack: node_type, node_payload_ptr, key, value_ptr, retdest

    DUP1 %eq_const(@SMT_NODE_HASH)        %jumpi(smt_insert_hash)
    DUP1 %eq_const(@SMT_NODE_INTERNAL)    %jumpi(smt_insert_internal)
    DUP1 %eq_const(@SMT_NODE_LEAF)        %jumpi(smt_insert_leaf)
global wtf0:
    PANIC

smt_insert_hash:
    // stack: node_type, node_payload_ptr, key, value_ptr, retdest
    POP
    // stack: node_payload_ptr, key, value_ptr, retdest
    %mload_trie_data
    // stack: hash, key, value_ptr, retdest
    ISZERO %jumpi(smt_insert_empty)
global wtf1:
    PANIC // Trying to insert in a non-empty hash node.
smt_insert_empty:
    // stack: key, value_ptr, retdest
    %get_trie_data_size
    // stack: index, key, value_ptr, retdest
    PUSH @SMT_NODE_LEAF %append_to_trie_data
    %stack (index, key, value_ptr) -> (value_ptr, key, value_ptr, index)
    %mstore_trie_data
    // stack: value_ptr, index, retdest
    %append_to_trie_data
    // stack: index, retdest
    SWAP1 JUMP

smt_insert_internal:
    // stack: node_type, node_payload_ptr, key, value_ptr, retdest
    POP
    // stack: node_payload_ptr, key, value_ptr, retdest
    SWAP1
    // stack: key, node_payload_ptr, value_ptr, retdest
    %pop_bit
    %stack (bit, key, node_payload_ptr, value_ptr, retdest) -> (bit, node_payload_ptr, node_payload_ptr, key, value_ptr, smt_insert_internal_after, retdest)
    ADD
    // stack: child_ptr_ptr, node_payload_ptr, key, value_ptr, smt_insert_internal_after, retdest
    DUP1 %mload_trie_data
     %stack (child_ptr, child_ptr_ptr, node_payload_ptr, key, value_ptr, smt_insert_internal_after) -> (child_ptr, key, value_ptr, smt_insert_internal_after, child_ptr_ptr, node_payload_ptr)
    %jump(smt_insert)

smt_insert_internal_after:
    // stack: new_node_ptr, child_ptr_ptr, node_payload_ptr, retdest
    SWAP1 %mstore_trie_data
    // stack: node_payload_ptr retdest
    %decrement
    SWAP1 JUMP

smt_insert_leaf:
    // stack: node_type, node_payload_ptr_ptr, key, value_ptr, retdest
    POP
    %stack (node_payload_ptr_ptr, key) -> (node_payload_ptr_ptr, key, node_payload_ptr_ptr, key)
    %mload_trie_data %mload_trie_data EQ %jumpi(smt_insert_leaf_same_key)
    // stack: node_payload_ptr_ptr, key, value_ptr, retdest
    // We create an internal node with two empty children, and then we insert the two leaves.
    %get_trie_data_size
    // stack: index, node_payload_ptr_ptr, key, value_ptr, retdest
    PUSH @SMT_NODE_INTERNAL %append_to_trie_data
    PUSH 0 %append_to_trie_data // Empty hash node
    PUSH 0 %append_to_trie_data // Empty hash node
    %stack (index, node_payload_ptr_ptr, key, value_ptr) -> (index, key, value_ptr, after_first_leaf, node_payload_ptr_ptr)
    %jump(smt_insert)
after_first_leaf:
    // stack: internal_ptr, node_payload_ptr_ptr, retdest
    SWAP1
    // stack: node_payload_ptr_ptr, internal_ptr, retdest
    %mload_trie_data DUP1 %mload_trie_data
    %stack (key, node_payload_ptr, internal_ptr) -> (internal_ptr, key, node_payload_ptr, after_second_leaf)
    %jump(smt_insert)
after_second_leaf:
    // stack: internal_ptr, retdest
    SWAP1 JUMP


global smt_insert_leaf_same_key:
    PANIC
    // stack: node_payload_ptr_ptr, key, value_ptr, retdest
    //DUP3 %increment %mload_trie_data POP
    %mload_trie_data
    // stack: key_ptr, key, value_ptr, retdest
    %increment
    //DUP1 %mload_trie_data POP
    %stack (value_ptr_ptr, key, value_ptr, retdest) -> (value_ptr_ptr, value_ptr, retdest)
    SWAP1 %increment SWAP1
    %mstore_trie_data
    JUMP
