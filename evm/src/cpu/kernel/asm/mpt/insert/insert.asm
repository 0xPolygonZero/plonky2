// Return a copy of the given node, with the given key set to the given value.
//
// Pre stack: node_ptr, num_nibbles, key, value_ptr, retdest
// Post stack: updated_node_ptr
global mpt_insert:
    // stack: node_ptr, num_nibbles, key, value_ptr, retdest
    DUP1 %mload_trie_data
    // stack: node_type, node_ptr, num_nibbles, key, value_ptr, retdest
    // Increment node_ptr, so it points to the node payload instead of its type.
    SWAP1 %increment SWAP1
    // stack: node_type, node_payload_ptr, num_nibbles, key, value_ptr, retdest

    DUP1 %eq_const(@MPT_NODE_EMPTY)     %jumpi(mpt_insert_empty)
    DUP1 %eq_const(@MPT_NODE_BRANCH)    %jumpi(mpt_insert_branch)
    DUP1 %eq_const(@MPT_NODE_EXTENSION) %jumpi(mpt_insert_extension)
    DUP1 %eq_const(@MPT_NODE_LEAF)      %jumpi(mpt_insert_leaf)

    // There's still the MPT_NODE_HASH case, but if we hit a hash node,
    // it means the prover failed to provide necessary Merkle data, so panic.
global mpt_insert_hash_node:
    PANIC

mpt_insert_empty:
    // stack: node_type, node_payload_ptr, num_nibbles, key, value_ptr, retdest
    %pop2
    // stack: num_nibbles, key, value_ptr, retdest
    // We will append a new leaf node to our MPT tape and return a pointer to it.
    %get_trie_data_size
    // stack: leaf_ptr, num_nibbles, key, value_ptr, retdest
    PUSH @MPT_NODE_LEAF %append_to_trie_data
    // stack: leaf_ptr, num_nibbles, key, value_ptr, retdest
    SWAP1 %append_to_trie_data
    // stack: leaf_ptr, key, value_ptr, retdest
    SWAP1 %append_to_trie_data
    // stack: leaf_ptr, value_ptr, retdest
    SWAP1 %append_to_trie_data
    // stack: leaf_ptr, retdest
    SWAP1
    JUMP

mpt_insert_branch:
    // stack: node_type, node_payload_ptr, num_nibbles, key, value_ptr, retdest
    POP

    //stack: node_payload_ptr, num_nibbles, key, value_ptr, retdest

    // At this point, we branch based on whether the key terminates with this branch node.
    // stack: node_payload_ptr, num_nibbles, key, value_ptr, retdest
    DUP2 %jumpi(mpt_insert_branch_nonterminal)

    // The key terminates here, so the value will be placed right in our (updated) branch node.
    // stack: node_payload_ptr, num_nibbles, key, value_ptr, retdest
    SWAP3
    // stack: value_ptr, num_nibbles, key, node_payload_ptr, retdest
    DUP4 %add_const(16)
    // stack: branch_value_ptr_ptr, value_ptr, num_nibbles, key, node_payload_ptr, retdest
    %mstore_trie_data
    // stack: num_nibbles, key, node_payload_ptr, retdest
    %pop2
    // stack: node_payload_ptr, retdest
    PUSH 1 SWAP1 SUB 
    // stack: branch_ptr, retdest
    SWAP1
    JUMP

mpt_insert_branch_nonterminal:
    // The key continues, so we split off the first (most significant) nibble,
    // and recursively insert into the child associated with that nibble.
    // stack: node_payload_ptr, num_nibbles, key, value_ptr, retdest
    %stack (node_payload_ptr, num_nibbles, key) -> (num_nibbles, key, node_payload_ptr)
    %split_first_nibble
    // stack: first_nibble, num_nibbles, key, node_payload_ptr, value_ptr, retdest
    DUP4 ADD
    // stack: child_ptr_ptr, num_nibbles, key, node_payload_ptr, value_ptr, retdest
    // Replace node_payload_ptr with branch pointer
    SWAP3 PUSH 1 SWAP1 SUB SWAP3
    %stack (child_ptr_ptr, num_nibbles, key, updated_branch_ptr, value_ptr)
        -> (child_ptr_ptr, num_nibbles, key, value_ptr,
            mpt_insert_branch_nonterminal_after_recursion,
            child_ptr_ptr, updated_branch_ptr)
    %mload_trie_data // Deref child_ptr_ptr, giving child_ptr
    %jump(mpt_insert)

mpt_insert_branch_nonterminal_after_recursion:
    // stack: updated_child_ptr, child_ptr_ptr, updated_branch_ptr, retdest
    SWAP1 %mstore_trie_data // Store the pointer to the updated child.
    // stack: updated_branch_ptr, retdest
    SWAP1
    JUMP
