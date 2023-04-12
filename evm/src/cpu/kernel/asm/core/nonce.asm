// Increment the nonce of the given account.
// Pre stack: address, retdest
// Post stack: (empty)

global get_nonce:
    // stack: address, retdest
    // TODO: Replace with actual implementation.
    POP
    JUMP

// Convenience macro to call get_nonce and return where we left off.
%macro get_nonce
    %stack (address) -> (address, %%after)
    %jump(get_nonce)
%%after:
%endmacro

// Increment the given account's nonce. Assumes the account already exists; panics otherwise.
global increment_nonce:
    // stack: address, retdest
    %mpt_read_state_trie
    // stack: account_ptr, retdest
    DUP1 ISZERO %jumpi(panic)
    // stack: nonce_ptr, retdest
    DUP1 %mload_trie_data
    // stack: nonce, nonce_ptr, retdest
    %increment
    SWAP1
    // stack: nonce_ptr, nonce', retdest
    %mstore_trie_data
    // stack: retdest
    JUMP

// Convenience macro to call increment_nonce and return where we left off.
%macro increment_nonce
    %stack (address) -> (address, %%after)
    %jump(increment_nonce)
%%after:
%endmacro
