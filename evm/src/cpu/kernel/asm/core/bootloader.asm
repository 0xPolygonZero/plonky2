// Loads some prover-provided contract code into the code segment of memory,
// then hashes the code and returns the hash.
global bootload_contract:
    // stack: address, retdest
//    %stack (address, retdest) -> (address, after_load_code, retdest)
//    %jump(load_code)
    PANIC // TODO

global bootload_code:
    // stack: code_len, retdest
    PANIC // TODO

    // stack: code_hash, retdest
    SWAP1
    JUMP
