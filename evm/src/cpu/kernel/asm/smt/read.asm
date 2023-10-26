// Given an address, return a pointer to the associated account data, which
// consists of four words (nonce, balance, storage_root, code_hash), in the
// state trie. Returns null if the address is not found.
global smt_read_state:
    // stack: addr, retdest
    %addr_to_state_key
    // stack: key, retdest
    %mload_global_metadata(@GLOBAL_METADATA_STATE_TRIE_ROOT) // node_ptr
    // stack: node_ptr, key, retdest
    %jump(smt_read)

// Convenience macro to call mpt_read_state_trie and return where we left off.
%macro smt_read_state
    %stack (addr) -> (addr, %%after)
    %jump(smt_read_state)
%%after:
%endmacro

global smt_read:
    // stack: node_ptr, key, retdest
    DUP1 %mload_trie_data
    // stack: node_type, node_ptr, key, retdest
    // Increment node_ptr, so it points to the node payload instead of its type.
    SWAP1 %increment SWAP1
    // stack: node_type, node_payload_ptr, key, retdest

    DUP1 %eq_const(@SMT_NODE_HASH)      %jumpi(smt_read_hash)
    DUP1 %eq_const(@SMT_NODE_INTERNAL)  %jumpi(smt_read_internal)
    DUP1 %eq_const(@SMT_NODE_LEAF)      %jumpi(smt_read_leaf)

global smt_read_hash:
    // stack: node_type, node_payload_ptr, key, retdest
    POP
    // stack: node_payload_ptr, key, retdest
    %mload_trie_data
    // stack: hash, key, retdest
    ISZERO %jumpi(smt_read_empty)
    PANIC // Trying to read a non-empty hash node. Should never happen.

global smt_read_empty:
    %stack (key, retdest) -> (retdest, 0)
    JUMP

global smt_read_internal:
    // stack: node_type, node_payload_ptr, key, retdest
    POP
    // stack: node_payload_ptr, key, retdest
    SWAP1
    // stack: key, node_payload_ptr, retdest
    %pop_bit
    %stack (bit, key, node_payload_ptr) -> (bit, node_payload_ptr, key)
    ADD
    // stack: child_ptr_ptr, key, retdest
    %mload_trie_data
    %jump(smt_read)

global smt_read_leaf:
    // stack: node_type, node_payload_ptr_ptr, key, retdest
    POP
    // stack: node_payload_ptr_ptr, key, retdest
    %mload_trie_data
    %stack (node_payload_ptr, key) -> (node_payload_ptr, key, node_payload_ptr)
    %mload_trie_data EQ %jumpi(smt_read_existing_leaf) // Checking if the key exists
global smt_read_non_existing_leaf:
    %stack (node_payload_ptr_ptr, retdest) -> (retdest, 0)
    JUMP

global smt_read_existing_leaf:
    // stack: node_payload_ptr_ptr, retdest
    %increment // We want to point to the account values, not the key.
    SWAP1 JUMP


