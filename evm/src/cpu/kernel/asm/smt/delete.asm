// Return a copy of the given node with the given key deleted.
// Assumes that the key is in the SMT.
//
// Pre stack: node_ptr, key, retdest
// Post stack: updated_node_ptr
global smt_delete:
    // stack: node_ptr, key, retdest
    DUP1 %mload_trie_data
    // stack: node_type, node_ptr, key, retdest
    // Increment node_ptr, so it points to the node payload instead of its type.
    SWAP1 %increment SWAP1
    // stack: node_type, node_payload_ptr, key, retdest

    DUP1 %eq_const(@SMT_NODE_INTERNAL)  %jumpi(smt_delete_internal)
    DUP1 %eq_const(@SMT_NODE_LEAF)      %jumpi(smt_delete_leaf)
    PANIC // Should never happen.

global smt_delete_leaf:
    // stack: node_type, node_payload_ptr, key, retdest
    %pop3
    PUSH 0 // empty node ptr
    SWAP1 JUMP

global smt_delete_internal:
    // stack: node_type, node_payload_ptr, key, retdest
    POP
    // stack: node_payload_ptr, key, retdest
    SWAP1 %pop_bit
    %stack (bit, key, node_payload_ptr, retdest) -> (bit, node_payload_ptr, key, internal_update, node_payload_ptr, bit, retdest)
    ADD
    // stack: child_ptr_ptr, key, internal_update, node_payload_ptr, bit, retdest
    %mload_trie_data
    // stack: child_ptr, key, internal_update, node_payload_ptr, bit, retdest
    %jump(smt_delete)

// Update the internal node, possibly deleting it, or returning a leaf node.
// TODO: Could replace a lot of `is_empty` check with just ISZERO.
global internal_update:
    // Update the child first.
    // stack: deleted_child_ptr, node_payload_ptr, bit, retdest
    DUP3 PUSH 1 SUB
    // stack: 1-bit, deleted_child_ptr, node_payload_ptr, bit, retdest
    DUP3 ADD
    // stack: sibling_ptr_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest
    %mload_trie_data DUP1 %mload_trie_data
    // stack: sibling_node_type, sibling_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest
    DUP1 %eq_const(@SMT_NODE_HASH) %jumpi(sibling_is_hash)
    %eq_const(@SMT_NODE_LEAF) %jumpi(sibling_is_leaf)
global sibling_is_internal:
    // stack: sibling_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest
    POP
global insert_child:
    // stack: deleted_child_ptr, node_payload_ptr, bit, retdest
    %stack (deleted_child_ptr, node_payload_ptr, bit) -> (node_payload_ptr, bit, deleted_child_ptr, node_payload_ptr)
    ADD %mstore_trie_data
    // stack: node_payload_ptr, retdest
    %decrement SWAP1
    // stack: retdest, node_ptr
    JUMP

global sibling_is_hash:
    // stack: sibling_node_type, sibling_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest
    POP
    // stack: sibling_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest
    %increment %mload_trie_data
    // stack: hash, deleted_child_ptr, node_payload_ptr, bit, retdest
    %jumpi(insert_child)
global sibling_is_empty:
    // stack: deleted_child_ptr, node_payload_ptr, bit, retdest
    DUP1 %mload_trie_data
    // stack: deleted_child_node_type, deleted_child_ptr, node_payload_ptr, bit, retdest
    DUP1 %eq_const(@SMT_NODE_HASH) %jumpi(sibling_is_empty_child_is_hash)
    DUP1 %eq_const(@SMT_NODE_LEAF) %jumpi(sibling_is_empty_child_is_leaf)
global sibling_is_empty_child_is_internal:
    // stack: deleted_child_node_type, deleted_child_ptr, node_payload_ptr, bit, retdest
    POP
    // stack: deleted_child_ptr, node_payload_ptr, bit, retdest
    %jump(insert_child)

global sibling_is_empty_child_is_hash:
    // stack: deleted_child_node_type, deleted_child_ptr, node_payload_ptr, bit, retdest
    POP
    // stack: deleted_child_ptr, node_payload_ptr, bit, retdest
    DUP1 %increment %mload_trie_data
    // stack: hash, deleted_child_ptr, node_payload_ptr, bit, retdest
    %jumpi(insert_child)
global sibling_is_empty_child_is_empty:
    // We can just delete this node.
    // stack: deleted_child_ptr, node_payload_ptr, bit, retdest
    %pop3
    SWAP1 PUSH 0
    // stack: retdest, 0
    JUMP

global sibling_is_empty_child_is_leaf:
    // stack: deleted_child_node_type, deleted_child_ptr, node_payload_ptr, bit, retdest
    POP
    // stack: deleted_child_ptr, node_payload_ptr, bit, retdest
    DUP1 %increment %mload_trie_data
    // stack: child_key_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest
    DUP1 %mload_trie_data
    // stack: key, child_key_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest
    %shl_const(1)
    // stack: key<<1, child_key_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest
    DUP5 ADD
    // stack: new_key, child_key_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest
    SWAP1 %mstore_trie_data
    %stack (deleted_child_ptr, node_payload_ptr, bit, retdest) -> (retdest, deleted_child_ptr)
    JUMP

global sibling_is_leaf:
    // stack: sibling_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest
    DUP2 %is_non_empty_node
    // stack: child_is_non_empty, sibling_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest
    %jumpi(sibling_is_leaf_child_is_non_empty)
global sibling_is_leaf_child_is_empty:
    // stack: sibling_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest
    DUP1 %increment %mload_trie_data
    // stack: sibling_key_ptr, sibling_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest
    DUP1 %mload_trie_data
    // stack: sibling_key, sibling_key_ptr, sibling_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest
    %shl_const(1)
    // stack: sibling_key<<1, sibling_key_ptr, sibling_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest
    DUP6 PUSH 1 SUB
    // stack: 1-bit, sibling_key<<1, sibling_key_ptr, sibling_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest
    ADD SWAP1 %mstore_trie_data
    // stack: sibling_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest
    %stack (sibling_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest) -> (retdest, sibling_ptr)
    JUMP

global sibling_is_leaf_child_is_non_empty:
    // stack: sibling_ptr, deleted_child_ptr, node_payload_ptr, bit, retdest
    POP
    // stack: deleted_child_ptr, node_payload_ptr, bit, retdest
    %jump(insert_child)
