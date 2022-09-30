global mpt_hash:
    // stack: node_ptr, retdest
    DUP1
    %mload_trie_data
    // stack: node_type, node_ptr, retdest
    // Increment node_ptr, so it points to the node payload instead of its type.
    SWAP1 %add_const(1) SWAP1
    // stack: node_type, node_payload_ptr, retdest

    DUP1 %eq_const(@MPT_NODE_EMPTY)     %jumpi(mpt_hash_empty)
    DUP1 %eq_const(@MPT_NODE_HASH)      %jumpi(mpt_hash_hash)
    DUP1 %eq_const(@MPT_NODE_BRANCH)    %jumpi(mpt_hash_branch)
    DUP1 %eq_const(@MPT_NODE_EXTENSION) %jumpi(mpt_hash_extension)
    DUP1 %eq_const(@MPT_NODE_LEAF)      %jumpi(mpt_hash_leaf)
    PANIC // Invalid node type? Shouldn't get here.

mpt_hash_empty:
    %stack (node_type, node_payload_ptr, retdest) -> (retdest, @EMPTY_NODE_HASH)
    JUMP

mpt_hash_hash:
    // stack: node_type, node_payload_ptr, retdest
    POP
    // stack: node_payload_ptr, retdest
    %mload_trie_data
    // stack: hash, retdest
    SWAP1
    JUMP

mpt_hash_branch:
    // stack: node_type, node_payload_ptr, retdest
    POP
    // stack: node_payload_ptr, retdest
    PANIC // TODO

mpt_hash_extension:
    // stack: node_type, node_payload_ptr, retdest
    POP
    // stack: node_payload_ptr, retdest
    PANIC // TODO

mpt_hash_leaf:
    // stack: node_type, node_payload_ptr, retdest
    POP
    // stack: node_payload_ptr, retdest
    DUP1 %mload_trie_data
    // stack: node_nibbles, node_payload_ptr, retdest
    PANIC // TODO
