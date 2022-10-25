/*
Insert into a leaf node.
The high-level logic can be expressed with the following pseudocode:

if node_len == insert_len && node_key == insert_key:
    return Leaf[node_key, insert_value]

common_len, common_key, node_len, node_key, insert_len, insert_key =
    split_common_prefix(node_len, node_key, insert_len, insert_key)

branch = [MPT_TYPE_BRANCH] + [0] * 17

// Process the node's entry.
if node_len > 0:
    node_key_first, node_len, node_key = split_first_nibble(node_len, node_key)
    branch[node_key_first + 1] = [MPT_TYPE_LEAF, node_len, node_key, node_value]
else:
    branch[17] = node_value

// Process the inserted entry.
if insert_len > 0:
    insert_key_first, insert_len, insert_key = split_first_nibble(insert_len, insert_key)
    branch[insert_key_first + 1] = [MPT_TYPE_LEAF, insert_len, insert_key, insert_value]
else:
    branch[17] = insert_value

// Add an extension node if there is a common prefix.
if common_len > 0:
    return [MPT_TYPE_EXTENSION, common_len, common_key, branch]
else:
    return branch
*/

global mpt_insert_leaf:
    // stack: node_type, node_payload_ptr, insert_len, insert_key, insert_value_ptr, retdest
    POP
    // stack: node_payload_ptr, insert_len, insert_key, insert_value_ptr, retdest
    %stack (node_payload_ptr, insert_len, insert_key) -> (insert_len, insert_key, node_payload_ptr)
    // stack: insert_len, insert_key, node_payload_ptr, insert_value_ptr, retdest
    DUP3 %increment %mload_trie_data
    // stack: node_key, insert_len, insert_key, node_payload_ptr, insert_value_ptr, retdest
    DUP4 %mload_trie_data
    // stack: node_len, node_key, insert_len, insert_key, node_payload_ptr, insert_value_ptr, retdest

    // If the keys match, i.e. node_len == insert_len && node_key == insert_key,
    // then we're simply replacing the leaf node's value. Since this is a common
    // case, it's best to detect it early. Calling %split_common_prefix could be
    // expensive as leaf keys tend to be long.
    DUP1 DUP4 EQ // node_len == insert_len
    DUP3 DUP6 EQ // node_key == insert_key
    MUL // Cheaper than AND
    // stack: keys_match, node_len, node_key, insert_len, insert_key, node_payload_ptr, insert_value_ptr, retdest
    %jumpi(keys_match)

    // Replace node_payload_ptr with node_value, which is node_payload[2].
    // stack: node_len, node_key, insert_len, insert_key, node_payload_ptr, insert_value_ptr, retdest
    SWAP4
    %add_const(2)
    %mload_trie_data
    SWAP4
    // stack: node_len, node_key, insert_len, insert_key, node_value_ptr, insert_value_ptr, retdest

    // Split off any common prefix between the node key and the inserted key.
    %split_common_prefix
    // stack: common_len, common_key, node_len, node_key, insert_len, insert_key, node_value_ptr, insert_value_ptr, retdest

    // For the remaining cases, we will need a new branch node since the two keys diverge.
    // We may also need an extension node above it (if common_len > 0); we will handle that later.
    // For now, we allocate the branch node, initially with no children or value.
    %get_trie_data_size  // pointer to the branch node we're about to create
    PUSH @MPT_NODE_BRANCH %append_to_trie_data
    %rep 17
        PUSH 0 %append_to_trie_data
    %endrep
    // stack: branch_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_value_ptr, insert_value_ptr, retdest

    // Now, we branch based on whether each key continues beyond the common
    // prefix, starting with the node key.

process_node_entry:
    DUP4 // node_len
    %jumpi(node_key_continues)

    // stack: branch_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_value_ptr, insert_value_ptr, retdest
    // branch[17] = node_value_ptr
    DUP8 // node_value_ptr
    DUP2 // branch_ptr
    %add_const(17)
    %mstore_trie_data

process_inserted_entry:
    DUP6 // insert_len
    %jumpi(insert_key_continues)

    // stack: branch_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_value_ptr, insert_value_ptr, retdest
    // branch[17] = insert_value_ptr
    DUP9 // insert_value_ptr
    DUP2 // branch_ptr
    %add_const(17)
    %mstore_trie_data

maybe_add_extension_for_common_key:
    // stack: branch_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_value_ptr, insert_value_ptr, retdest
    // If common_len > 0, we need to add an extension node.
    DUP2 %jumpi(add_extension_for_common_key)
    // Otherwise, we simply return branch_ptr.
    SWAP8
    %pop8
    // stack: branch_ptr, retdest
    SWAP1
    JUMP

add_extension_for_common_key:
    // stack: branch_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_value_ptr, insert_value_ptr, retdest
    // Pseudocode: return [MPT_TYPE_EXTENSION, common_len, common_key, branch]
    %get_trie_data_size // pointer to the extension node we're about to create
    // stack: extension_ptr, branch_ptr, common_len, common_key, ...
    PUSH @MPT_NODE_EXTENSION %append_to_trie_data
    SWAP2 %append_to_trie_data // Append common_len to our node
    // stack: branch_ptr, extension_ptr, common_key, ...
    SWAP2 %append_to_trie_data // Append common_key to our node
    // stack: extension_ptr, branch_ptr, ...
    SWAP1 %append_to_trie_data // Append branch_ptr to our node
    // stack: extension_ptr, node_len, node_key, insert_len, insert_key, node_value_ptr, insert_value_ptr, retdest
    SWAP6
    %pop6
    // stack: extension_ptr, retdest
    SWAP1
    JUMP

node_key_continues:
    // stack: branch_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_value_ptr, insert_value_ptr, retdest
    // branch[node_key_first + 1] = Leaf[node_len, node_key, node_value]
    // To minimize stack manipulation, we won't actually mutate the node_len, node_key variables in our stack.
    // Instead we will duplicate them, and leave the old ones alone; they won't be used.
    DUP5 DUP5
    // stack: node_len, node_key, branch_ptr, ...
    %split_first_nibble
    // stack: node_key_first, node_len, node_key, branch_ptr, ...
    %get_trie_data_size // pointer to the leaf node we're about to create
    // stack: leaf_ptr, node_key_first, node_len, node_key, branch_ptr, ...
    SWAP1
    DUP5 // branch_ptr
    %increment // Skip over node type field
    ADD // Add node_key_first
    %mstore_trie_data
    // stack: node_len, node_key, branch_ptr, ...
    PUSH @MPT_NODE_LEAF %append_to_trie_data
    %append_to_trie_data // Append node_len to our leaf node
    %append_to_trie_data // Append node_key to our leaf node
    // stack: branch_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_value_ptr, insert_value_ptr, retdest
    DUP8 %append_to_trie_data // Append node_value_ptr to our leaf node
    %jump(process_inserted_entry)

insert_key_continues:
    // stack: branch_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_value_ptr, insert_value_ptr, retdest
    // branch[insert_key_first + 1] = Leaf[insert_len, insert_key, insert_value]
    // To minimize stack manipulation, we won't actually mutate the insert_len, insert_key variables in our stack.
    // Instead we will duplicate them, and leave the old ones alone; they won't be used.
    DUP7 DUP7
    // stack: insert_len, insert_key, branch_ptr, ...
    %split_first_nibble
    // stack: insert_key_first, insert_len, insert_key, branch_ptr, ...
    %get_trie_data_size // pointer to the leaf node we're about to create
    // stack: leaf_ptr, insert_key_first, insert_len, insert_key, branch_ptr, ...
    SWAP1
    DUP5 // branch_ptr
    %increment // Skip over node type field
    ADD // Add insert_key_first
    %mstore_trie_data
    // stack: insert_len, insert_key, branch_ptr, ...
    PUSH @MPT_NODE_LEAF %append_to_trie_data
    %append_to_trie_data // Append insert_len to our leaf node
    %append_to_trie_data // Append insert_key to our leaf node
    // stack: branch_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_value_ptr, insert_value_ptr, retdest
    DUP9 %append_to_trie_data // Append insert_value_ptr to our leaf node
    %jump(maybe_add_extension_for_common_key)

keys_match:
    // The keys match exactly, so we simply create a new leaf node with the new value.xs
    // stack: node_len, node_key, insert_len, insert_key, node_payload_ptr, insert_value_ptr, retdest
    %stack (node_len, node_key, insert_len, insert_key, node_payload_ptr, insert_value_ptr)
        -> (node_len, node_key, insert_value_ptr)
    // stack: common_len, common_key, insert_value_ptr, retdest
    %get_trie_data_size // pointer to the leaf node we're about to create
    // stack: updated_leaf_ptr, common_len, common_key, insert_value_ptr, retdest
    PUSH @MPT_NODE_LEAF %append_to_trie_data
    SWAP1 %append_to_trie_data // Append common_len to our leaf node
    SWAP1 %append_to_trie_data // Append common_key to our leaf node
    SWAP1 %append_to_trie_data // Append insert_value_ptr to our leaf node
    // stack: updated_leaf_ptr, retdestx
    SWAP1
    JUMP
