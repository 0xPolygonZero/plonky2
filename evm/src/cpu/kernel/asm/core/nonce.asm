// Increment the nonce of the given account.
// Pre stack: address, retdest
// Post stack: (empty)

global get_nonce:
    // stack: address, retdest
    // TODO: Replace with actual implementation.
    JUMP

// Convenience macro to call get_nonce and return where we left off.
%macro get_nonce
    %stack (address) -> (address, %%after)
    %jump(get_nonce)
%%after:
%endmacro

global increment_nonce:
    // stack: address, retdest
    // TODO: Replace with actual implementation.
    POP
    JUMP

// Convenience macro to call increment_nonce and return where we left off.
%macro increment_nonce
    %stack (address) -> (address, %%after)
    %jump(increment_nonce)
%%after:
%endmacro
