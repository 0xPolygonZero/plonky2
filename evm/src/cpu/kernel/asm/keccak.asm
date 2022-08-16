// Computes the Keccak256 hash of some arbitrary bytes in memory.
// The given memory values should be in the range of a byte.
//
// Pre stack: ADDR, len, retdest
// Post stack: hash
global keccak_general:
    // stack: ADDR, len
    // TODO
