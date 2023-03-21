// Get the nonce of the given account.
// Pre stack: address, retdest
// Post stack: (empty)
global nonce:
    // stack: address, retdest
    %mpt_read_state_trie
    // stack: account_ptr, retdest
    // The nonce is the first account field, so we deref the account pointer itself.
    // Note: We don't need to handle account_ptr=0, as trie_data[0] = 0,
    // so the deref will give 0 (the default nonce) as desired.
    %mload_trie_data
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
    %mpt_read_state_trie
    // stack: account_ptr, retdest
    DUP1 ISZERO %jumpi(increment_nonce_no_such_account)
    // stack: nonce_ptr, retdest
    DUP1 %mload_trie_data
    // stack: nonce, nonce_ptr, retdest
    %increment
    SWAP1
    // stack: nonce_ptr, nonce', retdest
    %mstore_trie_data
    // stack: retdest
    JUMP
global increment_nonce_no_such_account:
    PANIC

// Convenience macro to call increment_nonce and return where we left off.
%macro increment_nonce
    %stack (address) -> (address, %%after)
    %jump(increment_nonce)
%%after:
%endmacro
