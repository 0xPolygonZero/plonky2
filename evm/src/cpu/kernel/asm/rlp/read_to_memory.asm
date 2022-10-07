// Read RLP data from the prover's tape, and save it to the SEGMENT_RLP_RAW
// segment of memory.

// Pre stack: retdest
// Post stack: (empty)

global read_rlp_to_memory:
    // stack: retdest
    PROVER_INPUT(rlp) // Read the RLP blob length from the prover tape.
    // stack: len, retdest
    PUSH 0 // initial position
    // stack: pos, len, retdest

read_rlp_to_memory_loop:
    // stack: pos, len, retdest
    DUP2
    DUP2
    EQ
    // stack: pos == len, pos, len, retdest
    %jumpi(read_rlp_to_memory_finish)
    // stack: pos, len, retdest
    PROVER_INPUT(rlp)
    // stack: byte, pos, len, retdest
    DUP2
    // stack: pos, byte, pos, len, retdest
    %mstore_current(@SEGMENT_RLP_RAW)
    // stack: pos, len, retdest
    %increment
    // stack: pos', len, retdest
    %jump(read_rlp_to_memory_loop)

read_rlp_to_memory_finish:
    // stack: pos, len, retdest
    %pop2
    // stack: retdest
    JUMP
