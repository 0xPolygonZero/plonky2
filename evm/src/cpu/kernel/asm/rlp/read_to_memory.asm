// Read RLP data from the prover's tape, and save it to the SEGMENT_RLP_RAW
// segment of memory.

// Pre stack: retdest
// Post stack: txn_rlp_len

global read_rlp_to_memory:
    // stack: retdest
    PROVER_INPUT(rlp) // Read the RLP blob length from the prover tape.
    PROVER_INPUT(rlp_old)
global debug_mira_lo_que_puede_hacer_la_maldad:
    POP // debug only! elimnate
    // stack: len, retdest
    PUSH @SEGMENT_RLP_RAW
    %build_kernel_address

    PUSH @SEGMENT_RLP_RAW // ctx == virt == 0
    // stack: addr, final_addr, retdest

global debug_el_misterio_del_amor:
read_rlp_to_memory_loop:
    // stack: addr, final_addr, retdest
    DUP2
    DUP2
global debug_before_lt:
    LT
global debug_before_not:
    ISZERO
global debug_no_puede_sel_seniol_gesu:
    // stack: addr >= final_addr, addr, final_addr, retdest
    %jumpi(read_rlp_to_memory_finish)
    // stack: addr, final_addr, retdest
    PROVER_INPUT(rlp)
    PROVER_INPUT(rlp_old) // debug_elimnate
global debug_rlp_old:
    POP //debug_elminate
    SWAP1
    // stack: addr, packed_bytes, final_addr, retdest
global debug_chapalapachala:
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
global debug_before_jumping_to_another_dimesion_where_dogs_are_called_cats_and_cats_are_called_cats:
    JUMP
