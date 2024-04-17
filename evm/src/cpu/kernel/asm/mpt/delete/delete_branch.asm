// Delete from a branch node.
// Algorithm is roughly:
//      - Delete `(num_nibbles-1, key[1:])` from `branch[key[0]]`.
//      - If the returned node is non-empty, update the branch node and return it.
//      - Otherwise, count the number of non-empty children of the branch node.
//          - If there are more than one, update the branch node and return it.
//          - If there is exactly one, transform the branch node into an leaf/extension node and return it.
// Assumes that `num_nibbles>0` and that the value of the branch node is zero.
// TODO: May need to revisit these assumptions depending on how the receipt trie is implemented.
global mpt_delete_branch:
    // stack: node_type, node_payload_ptr, num_nibbles, key, retdest
    POP
    // stack: node_payload_ptr, num_nibbles, key, retdest
    DUP2 ISZERO %jumpi(panic) // This should never happen.
    DUP3 DUP3
    // stack: num_nibbles, key, node_payload_ptr, num_nibbles, key, retdest
    %split_first_nibble
    %stack (first_nibble, num_nibbles, key, node_payload_ptr, old_num_nibbles, old_key) ->
        (node_payload_ptr, first_nibble, num_nibbles, key, after_mpt_delete_branch, first_nibble, node_payload_ptr)
    ADD
    // stack: child_ptr_ptr, num_nibbles, key, after_mpt_delete_branch, first_nibble, node_payload_ptr, retdest
    %mload_trie_data
    %jump(mpt_delete)

after_mpt_delete_branch:
    // stack: updated_child_ptr, first_nibble, node_payload_ptr, retdest
    // If the updated child is empty, check if we need to normalize the branch node.
    DUP1 %mload_trie_data ISZERO %jumpi(maybe_normalize_branch)

// Set `branch[first_nibble] = updated_child_ptr`.
update_branch:
    // stack: updated_child_ptr, first_nibble, node_payload_ptr, retdest
    DUP3 DUP3 ADD
    // stack: node_payload_ptr+first_nibble, updated_child_ptr, first_nibble, node_payload_ptr, retdest
    %mstore_trie_data
    %stack (first_nibble, node_payload_ptr, retdest) -> (node_payload_ptr, 1, retdest)
    SUB 
    // stack: node_ptr, retdest
    SWAP1
    JUMP

// The updated child is empty. Count how many non-empty children the branch node has.
// If it's one, transform the branch node into an leaf/extension node and return it.
maybe_normalize_branch:
    // stack: updated_child_ptr, first_nibble, node_payload_ptr, retdest
    PUSH 0 %mstore_kernel_general(0) PUSH 0 %mstore_kernel_general(1)
    // stack: updated_child_ptr, first_nibble, node_payload_ptr, retdest
    PUSH 0
// Loop from i=0..16 excluding `first_nibble` and store the number of non-empty children in
// KernelGeneral[0]. Also store the last non-empty child in KernelGeneral[1].
loop:
    // stack: i, updated_child_ptr, first_nibble, node_payload_ptr, retdest
    DUP1 DUP4 EQ %jumpi(loop_eq_first_nibble)
    // stack: i, updated_child_ptr, first_nibble, node_payload_ptr, retdest
    DUP1 %eq_const(16) %jumpi(loop_end)
    DUP1 DUP5 ADD %mload_trie_data %mload_trie_data ISZERO ISZERO %jumpi(loop_non_empty)
    // stack: i, updated_child_ptr, first_nibble, node_payload_ptr, retdest
    %increment %jump(loop)
loop_eq_first_nibble:
    // stack: i, updated_child_ptr, first_nibble, node_payload_ptr, retdest
    %increment %jump(loop)
loop_non_empty:
    // stack: i, updated_child_ptr, first_nibble, node_payload_ptr, retdest
    %mload_kernel_general(0) %increment %mstore_kernel_general(0)
    DUP1 %mstore_kernel_general(1)
    %increment %jump(loop)
loop_end:
    // stack: i, updated_child_ptr, first_nibble, node_payload_ptr, retdest
    POP
    // stack: updated_child_ptr, first_nibble, node_payload_ptr, retdest
    // If there's more than one non-empty child, simply update the branch node.
    %mload_kernel_general(0) %gt_const(1) %jumpi(update_branch)
    %mload_kernel_general(0) ISZERO %jumpi(panic) // This should never happen.
    // Otherwise, transform the branch node into a leaf/extension node.
    // stack: updated_child_ptr, first_nibble, node_payload_ptr, retdest
    %mload_kernel_general(1)
    // stack: i, updated_child_ptr, first_nibble, node_payload_ptr, retdest
    DUP4 ADD %mload_trie_data
    // stack: only_child_ptr, updated_child_ptr, first_nibble, node_payload_ptr, retdest
    DUP1 %mload_trie_data %eq_const(@MPT_NODE_BRANCH)     %jumpi(maybe_normalize_branch_branchhash)
    DUP1 %mload_trie_data %eq_const(@MPT_NODE_HASH)       %jumpi(maybe_normalize_branch_branchhash)
    DUP1 %mload_trie_data %eq_const(@MPT_NODE_EXTENSION)  %jumpi(maybe_normalize_branch_leafext)
    DUP1 %mload_trie_data %eq_const(@MPT_NODE_LEAF)       %jumpi(maybe_normalize_branch_leafext)
    PANIC // This should never happen.

// The only child of the branch node is a branch node or a hash node.
// Transform the branch node into an extension node of length 1.
// This assumes that the hash node does not contain a leaf or an extension node (in which case this implementation is incorrect).
maybe_normalize_branch_branchhash:
    // stack: only_child_ptr, updated_child_ptr, first_nibble, node_payload_ptr, retdest
    %get_trie_data_size // pointer to the extension node we're about to create
    // stack: extension_ptr, only_child_ptr, updated_child_ptr, first_nibble, node_payload_ptr, retdest
    PUSH @MPT_NODE_EXTENSION %append_to_trie_data
    // stack: extension_ptr, only_child_ptr, updated_child_ptr, first_nibble, node_payload_ptr, retdest
    PUSH 1 %append_to_trie_data // Append node_len to our node
    // stack: extension_ptr, only_child_ptr, updated_child_ptr, first_nibble, node_payload_ptr, retdest
    %mload_kernel_general(1) %append_to_trie_data // Append node_key to our node
    // stack: extension_ptr, only_child_ptr, updated_child_ptr, first_nibble, node_payload_ptr, retdest
    SWAP1 %append_to_trie_data // Append updated_child_node_ptr to our node
    %stack (extension_ptr, updated_child_ptr, first_nibble, node_payload_ptr, retdest) -> (retdest, extension_ptr)
    JUMP

// The only child of the branch node is a leaf/extension node.
// Transform the branch node into an leaf/extension node of length 1+len(child).
// For that, return the modified child as the new node.
maybe_normalize_branch_leafext:
    // stack: only_child_ptr, updated_child_ptr, first_nibble, node_payload_ptr, retdest
    DUP1 %increment %mload_trie_data
    // stack: child_len, only_child_ptr, updated_child_ptr, first_nibble, node_payload_ptr, retdest
    DUP2 %add_const(2) %mload_trie_data
    // stack: child_key, child_len, only_child_ptr, updated_child_ptr, first_nibble, node_payload_ptr, retdest
    %mload_kernel_general(1)
    %stack (i, child_key, child_len, only_child_ptr, updated_child_ptr, first_nibble, node_payload_ptr) ->
        (1, i, child_len, child_key, only_child_ptr)
    %merge_nibbles
    // stack: len, key, only_child_ptr,retdest
    DUP3
    // stack: node_ptr, len, key, only_child_ptr, retdest
    SWAP1 DUP2
    // stack: node_ptr, len, node_ptr, key, only_child_ptr, retdest
    %increment %mstore_trie_data // Change len in the child node
    // stack: node_ptr, key, only_child_ptr, retdest
    %add_const(2) %mstore_trie_data // Change key in the child node
    // stack: node_ptr, retdest
    SWAP1 JUMP
