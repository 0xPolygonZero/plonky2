// Get the nonce of the given account.
// Pre stack: address, retdest
// Post stack: (empty)
global nonce:
    // stack: address, retdest
    %key_nonce
    %smt_read_state %mload_trie_data
    // stack: nonce, retdest
    SWAP1 JUMP

// Convenience macro to call nonce and return where we left off.
%macro nonce
    %stack (address) -> (address, %%after)
    %jump(nonce)
%%after:
%endmacro

// Increment the given account's nonce. Assumes the account already exists; panics otherwise.
global increment_nonce:
    // stack: address, retdest
    DUP1
    %key_nonce %smt_read_state
    // stack: nonce_ptr, address, retdest
    DUP1 ISZERO %jumpi(create_nonce)
    // stack: nonce_ptr, address, retdest
    DUP1 %mload_trie_data
    // stack: nonce, nonce_ptr, address, retdest
    DUP1 DUP4 %journal_add_nonce_change
    // stack: nonce, nonce_ptr, address, retdest
    %increment
    SWAP1
    // stack: nonce_ptr, nonce', address, retdest
    %mstore_trie_data
    // stack: address, retdest
    POP
    JUMP

create_nonce:
    // stack: nonce_ptr, address, retdest
    POP
    // stack: address, retdest
    PUSH 0 DUP2 %journal_add_nonce_change
    // stack: address, retdest
    %key_nonce
    %stack (key_nonce) -> (key_nonce, 1)
    %jump(smt_insert_state)

// Convenience macro to call increment_nonce and return where we left off.
%macro increment_nonce
    %stack (address) -> (address, %%after)
    %jump(increment_nonce)
%%after:
%endmacro
