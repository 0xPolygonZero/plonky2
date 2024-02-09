// Return a copy of the given node with the given key deleted.
// Assumes that the key is in the trie.
//
// Pre stack: node_ptr, num_nibbles, key, retdest
// Post stack: updated_node_ptr
global mpt_delete:
    // stack: node_ptr, num_nibbles, key, retdest
    DUP1 %mload_trie_data
    // stack: node_type, node_ptr, num_nibbles, key, retdest
    // Increment node_ptr, so it points to the node payload instead of its type.
    SWAP1 %increment SWAP1
    // stack: node_type, node_payload_ptr, num_nibbles, key, retdest

    DUP1 %eq_const(@MPT_NODE_BRANCH)    %jumpi(mpt_delete_branch)
    DUP1 %eq_const(@MPT_NODE_EXTENSION) %jumpi(mpt_delete_extension)
    DUP1 %eq_const(@MPT_NODE_LEAF)      %jumpi(mpt_delete_leaf)
         %eq_const(@MPT_NODE_EMPTY)     %jumpi(panic) // This should never happen.
    PANIC

mpt_delete_leaf:
    // stack: node_type, node_payload_ptr, num_nibbles, key, retdest
    %pop4
    PUSH 0 // empty node ptr
    SWAP1 JUMP

/*
global delete_account:
    %stack (address, retdest) -> (address, retdest)
    DUP1 %key_nonce
    // stack: key_nonce, address, retdest
    DUP1 %smt_read_state ISZERO %jumpi(zero_nonce)
    // stack: key_nonce, address, retdest
    DUP1 %smt_delete_state
    // stack: key_nonce, address, retdest
zero_nonce:
    // stack: key_nonce, address, retdest
    POP
    // stack: address, retdest
    DUP1 %key_balance
    // stack: key_balance, address, retdest
    DUP1 %smt_read_state ISZERO %jumpi(zero_balance)
    // stack: key_balance, address, retdest
    DUP1 %smt_delete_state
    // stack: key_balance, address, retdest
zero_balance:
    // stack: key_balance, address, retdest
    POP
    // stack: address, retdest
    DUP1 %key_code
    // stack: key_code, address, retdest
    DUP1 %smt_read_state ISZERO %jumpi(zero_code)
    // stack: key_code, address, retdest
    DUP1 %smt_delete_state
    // stack: key_code, address, retdest
zero_code_length:
    // stack: key_code, address, retdest
    POP
    // stack: address, retdest
    DUP1 %key_code_length
    // stack: key_code_length, address, retdest
    DUP1 %smt_read_state ISZERO %jumpi(zero_code)
    // stack: key_code_length, address, retdest
    %smt_delete_state
    // stack: address, retdest
    // N.B.: We don't delete the storage, since there's no way of knowing keys used.
    POP JUMP

%macro delete_account
    %stack (address) -> (address, %%after)
    %jump(delete_account)
%%after:
    // stack: (empty)
%endmacro
*/