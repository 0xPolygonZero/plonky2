// Computes the address of a contract based on the conventional scheme, i.e.
//     address = KEC(RLP(sender, nonce))[12:]
//
// Pre stack: sender, nonce, retdest
// Post stack: address
global get_create_address:
    // stack: sender, nonce, retdest
    // TODO: Replace with actual implementation.
    %pop2
    PUSH 123
    // stack: address, retdest
    %observe_new_address
    SWAP1
    JUMP

// Computes the address for a contract based on the CREATE2 rule, i.e.
//     address = KEC(0xff || sender || salt || code_hash)[12:]
//
// Pre stack: sender, salt, code_hash, retdest
// Post stack: address
global get_create2_address:
    // stack: sender, salt, code_hash, retdest
    // TODO: Replace with actual implementation.
    %pop3
    PUSH 123
    // stack: address, retdest
    %observe_new_address
    SWAP1
    JUMP

// This should be called whenever a new address is created. This is only for debugging. It does
// nothing, but just provides a single hook where code can react to newly created addresses.
global observe_new_address:
    // stack: address, retdest
    SWAP1
    // stack: retdest, address
    JUMP

// Convenience macro to call observe_new_address and return where we left off.
%macro observe_new_address
    %stack (address) -> (address, %%after)
    %jump(observe_new_address)
%%after:
%endmacro
