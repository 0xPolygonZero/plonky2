/*
Insert into an extension node.
The high-level logic can be expressed with the following pseudocode:

common_len, common_key, node_len, node_key, insert_len, insert_key =
    split_common_prefix(node_len, node_key, insert_len, insert_key)

if node_len == 0:
    new_node = insert(node_child, insert_len, insert_key, insert_value)
else:
    new_node = [MPT_TYPE_BRANCH] + [0] * 17

    // Process the node's child.
    if node_len > 1:
        // The node key continues with multiple nibbles left, so we can't place
        // node_child directly in the branch, but need an extension for it.
        node_key_first, node_len, node_key = split_first_nibble(node_len, node_key)
        new_node[node_key_first + 1] = [MPT_TYPE_EXTENSION, node_len, node_key, node_child]
    else:
        // The remaining node_key is a single nibble, so we can place node_child directly in the branch.
        new_node[node_key + 1] = node_child

    // Process the inserted entry.
    if insert_len > 0:
        // The insert key continues. Add a leaf node for it.
        insert_key_first, insert_len, insert_key = split_first_nibble(insert_len, insert_key)
        new_node[insert_key_first + 1] = [MPT_TYPE_LEAF, insert_len, insert_key, insert_value]
    else:
        new_node[17] = insert_value

if common_len > 0:
    return [MPT_TYPE_EXTENSION, common_len, common_key, new_node]
else:
    return new_node
*/

global mpt_insert_extension:
    // stack: node_type, node_payload_ptr, insert_len, insert_key, insert_value_ptr, retdest
    POP
    // stack: node_payload_ptr, insert_len, insert_key, insert_value_ptr, retdest

    // We start by loading the extension node's three fields: node_len, node_key, node_child_ptr
    DUP1 %add_const(2) %mload_trie_data
    // stack: node_child_ptr, node_payload_ptr, insert_len, insert_key, insert_value_ptr, retdest
    %stack (node_child_ptr, node_payload_ptr, insert_len, insert_key)
        -> (node_payload_ptr, insert_len, insert_key, node_child_ptr)
    // stack: node_payload_ptr, insert_len, insert_key, node_child_ptr, insert_value_ptr, retdest
    DUP1 %increment %mload_trie_data
    // stack: node_key, node_payload_ptr, insert_len, insert_key, node_child_ptr, insert_value_ptr, retdest
    SWAP1 %mload_trie_data
    // stack: node_len, node_key, insert_len, insert_key, node_child_ptr, insert_value_ptr, retdest

    // Next, we split off any key prefix which is common to the node's key and the inserted key.
    %split_common_prefix
    // stack: common_len, common_key, node_len, node_key, insert_len, insert_key, node_child_ptr, insert_value_ptr, retdest

    // Now we branch based on whether the node key continues beyond the common prefix.
    DUP3 %jumpi(node_key_continues)

    // The node key does not continue. In this case we recurse. Pseudocode:
    //     new_node = insert(node_child, insert_len, insert_key, insert_value)
    // and then proceed to maybe_add_extension_for_common_key.
    // stack: common_len, common_key, node_len, node_key, insert_len, insert_key, node_child_ptr, insert_value_ptr, retdest
    PUSH maybe_add_extension_for_common_key
    DUP9 // insert_value_ptr
    DUP8 // insert_key
    DUP8 // insert_len
    DUP11 // node_child_ptr
    %jump(mpt_insert)

node_key_continues:
    // stack: common_len, common_key, node_len, node_key, insert_len, insert_key, node_child_ptr, insert_value_ptr, retdest
    // Allocate new_node, a branch node which is initially empty
    // Pseudocode: new_node = [MPT_TYPE_BRANCH] + [0] * 17
    %get_trie_data_size // pointer to the branch node we're about to create
    PUSH @MPT_NODE_BRANCH %append_to_trie_data
    %rep 17
        PUSH 0 %append_to_trie_data
    %endrep

process_node_child:
    // stack: new_node_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_child_ptr, insert_value_ptr, retdest
    // We want to check if node_len > 1. We already know node_len > 0 since we're in node_key_continues,
    // so it suffices to check 1 - node_len != 0
    DUP4 // node_len
    PUSH 1 SUB
    %jumpi(node_key_continues_multiple_nibbles)

    // If we got here, node_len = 1.
    // Pseudocode: new_node[node_key + 1] = node_child
    // stack: new_node_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_child_ptr, insert_value_ptr, retdest
    DUP8 // node_child_ptr
    DUP2 // new_node_ptr
    %increment
    DUP7 // node_key
    ADD
    %mstore_trie_data
    // stack: new_node_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_child_ptr, insert_value_ptr, retdest
    %jump(process_inserted_entry)

node_key_continues_multiple_nibbles:
    // stack: new_node_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_child_ptr, insert_value_ptr, retdest
    // Pseudocode: node_key_first, node_len, node_key = split_first_nibble(node_len, node_key)
    // To minimize stack manipulation, we won't actually mutate the node_len, node_key variables in our stack.
    // Instead we will duplicate them, and leave the old ones alone; they won't be used.
    DUP5 DUP5
    // stack: node_len, node_key, new_node_ptr, ...
    %split_first_nibble
    // stack: node_key_first, node_len, node_key, new_node_ptr, ...

    // Pseudocode: new_node[node_key_first + 1] = [MPT_TYPE_EXTENSION, node_len, node_key, node_child]
    %get_trie_data_size // pointer to the extension node we're about to create
    // stack: ext_node_ptr, node_key_first, node_len, node_key, new_node_ptr, ...
    PUSH @MPT_NODE_EXTENSION %append_to_trie_data
    // stack: ext_node_ptr, node_key_first, node_len, node_key, new_node_ptr, ...
    SWAP2 %append_to_trie_data // Append node_len
    // stack: node_key_first, ext_node_ptr, node_key, new_node_ptr, ...
    SWAP2 %append_to_trie_data // Append node_key
    // stack: ext_node_ptr, node_key_first, new_node_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_child_ptr, insert_value_ptr, retdest
    DUP10 %append_to_trie_data // Append node_child_ptr

    SWAP1
    // stack: node_key_first, ext_node_ptr, new_node_ptr, ...
    DUP3 // new_node_ptr
    ADD
    %increment
    // stack: new_node_ptr + node_key_first + 1, ext_node_ptr, new_node_ptr, ...
    %mstore_trie_data
    %jump(process_inserted_entry)

process_inserted_entry:
    // stack: new_node_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_child_ptr, insert_value_ptr, retdest
    DUP6 // insert_len
    %jumpi(insert_key_continues)

    // If we got here, insert_len = 0, so we store the inserted value directly in our new branch node.
    // Pseudocode: new_node[17] = insert_value
    DUP9 // insert_value_ptr
    DUP2 // new_node_ptr
    %add_const(17)
    %mstore_trie_data
    %jump(maybe_add_extension_for_common_key)

insert_key_continues:
    // stack: new_node_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_child_ptr, insert_value_ptr, retdest
    // Pseudocode: insert_key_first, insert_len, insert_key = split_first_nibble(insert_len, insert_key)
    // To minimize stack manipulation, we won't actually mutate the node_len, node_key variables in our stack.
    // Instead we will duplicate them, and leave the old ones alone; they won't be used.
    DUP7 DUP7
    // stack: insert_len, insert_key, new_node_ptr, ...
    %split_first_nibble
    // stack: insert_key_first, insert_len, insert_key, new_node_ptr, ...

    // Pseudocode: new_node[insert_key_first + 1] = [MPT_TYPE_LEAF, insert_len, insert_key, insert_value]
    %get_trie_data_size // pointer to the leaf node we're about to create
    // stack: leaf_node_ptr, insert_key_first, insert_len, insert_key, new_node_ptr, ...
    PUSH @MPT_NODE_LEAF %append_to_trie_data
    // stack: leaf_node_ptr, insert_key_first, insert_len, insert_key, new_node_ptr, ...
    SWAP2 %append_to_trie_data // Append insert_len
    // stack: insert_key_first, leaf_node_ptr, insert_key, new_node_ptr, ...
    SWAP2 %append_to_trie_data // Append insert_key
    // stack: leaf_node_ptr, insert_key_first, new_node_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_child_ptr, insert_value_ptr, retdest
    DUP11 %append_to_trie_data // Append insert_value_ptr

    SWAP1
    // stack: insert_key_first, leaf_node_ptr, new_node_ptr, ...
    DUP3 // new_node_ptr
    ADD
    %increment
    // stack: new_node_ptr + insert_key_first + 1, leaf_node_ptr, new_node_ptr, ...
    %mstore_trie_data
    %jump(maybe_add_extension_for_common_key)

maybe_add_extension_for_common_key:
    // stack: new_node_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_child_ptr, insert_value_ptr, retdest
    // If common_len > 0, we need to add an extension node.
    DUP2 %jumpi(add_extension_for_common_key)
    // Otherwise, we simply return new_node_ptr.
    SWAP8
    %pop8
    // stack: new_node_ptr, retdest
    SWAP1
    JUMP

add_extension_for_common_key:
    // stack: new_node_ptr, common_len, common_key, node_len, node_key, insert_len, insert_key, node_child_ptr, insert_value_ptr, retdest
    // Pseudocode: return [MPT_TYPE_EXTENSION, common_len, common_key, new_node]
    %get_trie_data_size // pointer to the extension node we're about to create
    // stack: extension_ptr, new_node_ptr, common_len, common_key, ...
    PUSH @MPT_NODE_EXTENSION %append_to_trie_data
    SWAP2 %append_to_trie_data // Append common_len to our node
    // stack: new_node_ptr, extension_ptr, common_key, ...
    SWAP2 %append_to_trie_data // Append common_key to our node
    // stack: extension_ptr, new_node_ptr, ...
    SWAP1 %append_to_trie_data // Append new_node_ptr to our node
    // stack: extension_ptr, node_len, node_key, insert_len, insert_key, node_child_ptr, insert_value_ptr, retdest
    SWAP6
    %pop6
    // stack: extension_ptr, retdest
    SWAP1
    JUMP
