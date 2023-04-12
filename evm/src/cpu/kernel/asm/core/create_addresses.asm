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
    SWAP1
    JUMP

// Computes the address for a contract based on the CREATE2 rule, i.e.
//     address = KEC(0xff || sender || salt || code_hash)[12:]
//
// Pre stack: sender, salt, CODE_ADDR, code_len, retdest
// Post stack: address
//
// Note: CODE_ADDR is a (context, segment, offset) tuple.
global get_create2_address:
    // stack: sender, salt, CODE_ADDR, code_len, retdest
    // TODO: Replace with actual implementation.
    %pop6
    PUSH 123
    SWAP1
    JUMP
