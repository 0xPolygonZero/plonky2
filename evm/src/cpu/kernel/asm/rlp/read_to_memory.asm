// Read RLP data from the prover's tape, and save it to the SEGMENT_RLP_RAW
// segment of memory.

// Pre stack: retdest
// Post stack: txn_rlp_len

global read_rlp_to_memory:
    // stack: retdest
    PROVER_INPUT(rlp) // Read the RLP blob length from the prover tape.
    // stack: len, retdest
    PUSH @SEGMENT_RLP_RAW
    %build_kernel_address

    PUSH @SEGMENT_RLP_RAW // ctx == virt == 0
    // stack: addr, final_addr, retdest
read_rlp_to_memory_loop:
    // stack: addr, final_addr, retdest
    DUP2
    DUP2
    LT
    ISZERO
    // stack: addr >= final_addr, addr, final_addr, retdest
    %jumpi(read_rlp_to_memory_finish)
    // stack: addr, final_addr, retdest
    PROVER_INPUT(rlp)
    SWAP1
    MSTORE_32BYTES_32
    // stack: addr', final_addr, retdest
    %jump(read_rlp_to_memory_loop)

read_rlp_to_memory_finish:
    // stack: addr, final_addr, retdest
    // we recover the offset here
    PUSH @SEGMENT_RLP_RAW // ctx == virt == 0
    DUP3 SUB
    // stack: pos, addr, final_addr, retdest
    %stack(pos, addr, final_addr, retdest) -> (retdest, pos)
    JUMP