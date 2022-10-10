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
    %get_trie_data_size
    // stack: updated_branch_ptr, node_type, node_payload_ptr, num_nibbles, key, value_ptr, retdest
    SWAP1
    %append_to_trie_data
    // stack: updated_branch_ptr, node_payload_ptr, num_nibbles, key, value_ptr, retdest
    SWAP1
    // stack: node_payload_ptr, updated_branch_ptr, num_nibbles, key, value_ptr, retdest

    // Copy the original node's data to our updated node.
    DUP1                %mload_trie_data %append_to_trie_data // Copy child[0]
    DUP1 %add_const(1)  %mload_trie_data %append_to_trie_data // ...
    DUP1 %add_const(2)  %mload_trie_data %append_to_trie_data
    DUP1 %add_const(3)  %mload_trie_data %append_to_trie_data
    DUP1 %add_const(4)  %mload_trie_data %append_to_trie_data
    DUP1 %add_const(5)  %mload_trie_data %append_to_trie_data
    DUP1 %add_const(6)  %mload_trie_data %append_to_trie_data
    DUP1 %add_const(7)  %mload_trie_data %append_to_trie_data
    DUP1 %add_const(8)  %mload_trie_data %append_to_trie_data
    DUP1 %add_const(9)  %mload_trie_data %append_to_trie_data
    DUP1 %add_const(10) %mload_trie_data %append_to_trie_data
    DUP1 %add_const(11) %mload_trie_data %append_to_trie_data
    DUP1 %add_const(12) %mload_trie_data %append_to_trie_data
    DUP1 %add_const(13) %mload_trie_data %append_to_trie_data
    DUP1 %add_const(14) %mload_trie_data %append_to_trie_data
    DUP1 %add_const(15) %mload_trie_data %append_to_trie_data // Copy child[15]
         %add_const(16) %mload_trie_data %append_to_trie_data // Copy value_ptr

    // At this point, we branch based on whether the key terminates with this branch node.
    // stack: updated_branch_ptr, num_nibbles, key, value_ptr, retdest
    DUP2 %jumpi(mpt_insert_branch_nonterminal)

    // The key terminates here, so the value will be placed right in our (updated) branch node.
    // stack: updated_branch_ptr, num_nibbles, key, value_ptr, retdest
    SWAP3
    // stack: value_ptr, num_nibbles, key, updated_branch_ptr, retdest
    DUP4 %add_const(17)
    // stack: updated_branch_value_ptr_ptr, value_ptr, num_nibbles, key, updated_branch_ptr, retdest
    %mstore_trie_data
    // stack: num_nibbles, key, updated_branch_ptr, retdest
    %pop2
    // stack: updated_branch_ptr, retdest
    SWAP1
    JUMP

mpt_insert_branch_nonterminal:
    // The key continues, so we split off the first (most significant) nibble,
    // and recursively insert into the child associated with that nibble.
    // stack: updated_branch_ptr, num_nibbles, key, value_ptr, retdest
    %stack (updated_branch_ptr, num_nibbles, key) -> (num_nibbles, key, updated_branch_ptr)
    %split_first_nibble
    // stack: first_nibble, num_nibbles, key, updated_branch_ptr, value_ptr, retdest
    DUP4 %increment ADD
    // stack: child_ptr_ptr, num_nibbles, key, updated_branch_ptr, value_ptr, retdest
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
