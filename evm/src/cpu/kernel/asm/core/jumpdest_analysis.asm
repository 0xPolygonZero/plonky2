// Set @SEGMENT_JUMPDEST_BITS to one between positions [init_pos, final_pos], 
// for the given context's code. Panics if we never hit final_pos
// Pre stack: init_pos, ctx, final_pos, retdest
// Post stack: (empty)
global verify_path:
loop_new:
    // stack: i, ctx, final_pos, retdest
    // Ideally we would break if i >= final_pos, but checking i > final_pos is
    // cheaper. It doesn't hurt to over-read by 1, since we'll read 0 which is
    // a no-op.
    DUP3 DUP2 EQ // i == final_pos
    %jumpi(return_new)
    DUP3 DUP2 GT // i > final_pos
    %jumpi(panic)

    // stack: i, ctx, final_pos, retdest
    %stack (i, ctx) -> (ctx, @SEGMENT_CODE, i, i, ctx)
    MLOAD_GENERAL
    // stack: opcode, i, ctx, final_pos, retdest

    DUP1 
    // Slightly more efficient than `%eq_const(0x5b) ISZERO`
    PUSH 0x5b
    SUB
    // stack: opcode != JUMPDEST, opcode, i, ctx, code_len, retdest
    %jumpi(continue_new)

    // stack: JUMPDEST, i, ctx, code_len, retdest
    %stack (JUMPDEST, i, ctx) -> (1, ctx, @SEGMENT_JUMPDEST_BITS, i, JUMPDEST, i, ctx)
    MSTORE_GENERAL

continue_new:
    // stack: opcode, i, ctx, code_len, retdest
    %add_const(code_bytes_to_skip)
    %mload_kernel_code
    // stack: bytes_to_skip, i, ctx, code_len, retdest
    ADD
    // stack: i, ctx, code_len, retdest
    %jump(loop_new)

return_new:
    // stack: i, ctx, code_len, retdest
    %pop3
    JUMP

// Populates @SEGMENT_JUMPDEST_BITS for the given context's code.
// Pre stack: ctx, code_len, retdest
// Post stack: (empty)
global jumpdest_analysis:
    // stack: ctx, code_len, retdest
    PUSH 0 // i = 0

loop:
    // stack: i, ctx, code_len, retdest
    // Ideally we would break if i >= code_len, but checking i > code_len is
    // cheaper. It doesn't hurt to over-read by 1, since we'll read 0 which is
    // a no-op.
    DUP3 DUP2 GT // i > code_len
    %jumpi(return)

    // stack: i, ctx, code_len, retdest
    %stack (i, ctx) -> (ctx, @SEGMENT_CODE, i, i, ctx)
    MLOAD_GENERAL
    // stack: opcode, i, ctx, code_len, retdest

    DUP1 
    // Slightly more efficient than `%eq_const(0x5b) ISZERO`
    PUSH 0x5b
    SUB
    // stack: opcode != JUMPDEST, opcode, i, ctx, code_len, retdest
    %jumpi(continue)

    // stack: JUMPDEST, i, ctx, code_len, retdest
    %stack (JUMPDEST, i, ctx) -> (1, ctx, @SEGMENT_JUMPDEST_BITS, i, JUMPDEST, i, ctx)
    MSTORE_GENERAL

continue:
    // stack: opcode, i, ctx, code_len, retdest
    %add_const(code_bytes_to_skip)
    %mload_kernel_code
    // stack: bytes_to_skip, i, ctx, code_len, retdest
    ADD
    // stack: i, ctx, code_len, retdest
    %jump(loop)

return:
    // stack: i, ctx, code_len, retdest
    %pop3
    JUMP

// Determines how many bytes away is the next opcode, based on the opcode we read.
// If we read a PUSH<n> opcode, next opcode is in n + 1 bytes, otherwise it's the next one.
//
// Note that the range of PUSH opcodes is [0x60, 0x80). I.e. PUSH1 is 0x60
// and PUSH32 is 0x7f.
code_bytes_to_skip:
    %rep 96
        BYTES 1 // 0x00-0x5f
    %endrep

    BYTES 2
    BYTES 3
    BYTES 4
    BYTES 5
    BYTES 6
    BYTES 7
    BYTES 8
    BYTES 9
    BYTES 10
    BYTES 11
    BYTES 12
    BYTES 13
    BYTES 14
    BYTES 15
    BYTES 16
    BYTES 17
    BYTES 18
    BYTES 19
    BYTES 20
    BYTES 21
    BYTES 22
    BYTES 23
    BYTES 24
    BYTES 25
    BYTES 26
    BYTES 27
    BYTES 28
    BYTES 29
    BYTES 30
    BYTES 31
    BYTES 32
    BYTES 33

    %rep 128
        BYTES 1 // 0x80-0xff
    %endrep


// A proof attesting that jumpdest is a valid jump destinations is
// either 0 or an index 0 < i <= jumpdest - 32.
// A proof is valid if:
// - i == 0 and we can go from the first opcode to jumpdest and code[jumpdest] = 0x5b
// - i > 0 and:
//     - for j in {i+0,..., i+31} code[j] != PUSHk for all k >= 32 - j - i,
//     - we can go from opcode i+32 to jumpdest,
//     - code[jumpdest] = 0x5b.
// stack: proof_prefix_addr, jumpdest, retdest
// stack: (empty) abort if jumpdest is not a valid destination
global is_jumpdest:
    GET_CONTEXT
    // stack: ctx, proof_prefix_addr, jumpdest, retdest
    %stack
        (ctx, proof_prefix_addr, jumpdest) ->
        (ctx, @SEGMENT_CODE, jumpdest, jumpdest, ctx, proof_prefix_addr)
    MLOAD_GENERAL
    // stack: opcode, jumpdest, ctx, proof_prefix_addr, retdest

    // Slightly more efficient than `%eq_const(0x5b) ISZERO`
    PUSH 0x5b
    SUB
    %jumpi(panic)

    //stack: jumpdest, ctx, proof_prefix_addr, retdest
    SWAP2 DUP1
    // stack: proof_prefix_addr, proof_prefix_addr, ctx, jumpdest
    %eq_const(0)
    %jumpi(verify_path)
    //stack: proof_prefix_addr, ctx, jumpdest, retdest
    // If we are here we need to check that the next 32 bytes are less
    // than JUMPXX for XX < 32 - i <=> opcode < 0x7f - i = 127 - i, 0 <= i < 32,
    // or larger than 127
    %check_and_step(127) %check_and_step(126) %check_and_step(125) %check_and_step(124)
    %check_and_step(123) %check_and_step(122) %check_and_step(121) %check_and_step(120)
    %check_and_step(119) %check_and_step(118) %check_and_step(117) %check_and_step(116)
    %check_and_step(115) %check_and_step(114) %check_and_step(113) %check_and_step(112)
    %check_and_step(111) %check_and_step(110) %check_and_step(109) %check_and_step(108)
    %check_and_step(107) %check_and_step(106) %check_and_step(105) %check_and_step(104)
    %check_and_step(103) %check_and_step(102) %check_and_step(101) %check_and_step(100)
    %check_and_step(99) %check_and_step(98) %check_and_step(97) %check_and_step(96)

    // check the remaining path
    %jump(verify_path)

return_is_jumpdest:
    //stack: proof_prefix_addr, jumpdest, retdest
    %pop2
    JUMP


// Chek if the opcode pointed by proof_prefix address is
// less than max and increment proof_prefix_addr
%macro check_and_step(max)
    %stack
        (proof_prefix_addr, ctx, jumpdest) ->
        (ctx, @SEGMENT_CODE, proof_prefix_addr, proof_prefix_addr, ctx, jumpdest)
    MLOAD_GENERAL
    // stack: opcode, proof_prefix_addr, ctx, jumpdest
    DUP1
    %gt_const(127)
    %jumpi(%%ok)
    %assert_lt_const($max)
    // stack: proof_prefix_addr, ctx, jumpdest
    PUSH 0 // We need something to pop
%%ok:
    POP
    %increment
%endmacro

%macro is_jumpdest
    %stack (proof, addr) -> (proof, addr, %%after)
    %jump(is_jumpdest)
%%after:
%endmacro

// Check if the jumpdest table is correct. This is done by
// non-deterministically guessing the sequence of jumpdest
// addresses used during program execution within the current context.
// For each jumpdest address we also non-deterministically guess
// a proof, which is another address in the code, such that 
// is_jumpdest don't abort when the proof is on the top of the stack
// an the jumpdest address below. If that's the case we set the
// corresponding bit in @SEGMENT_JUMPDEST_BITS to 1.
// 
// stack: retdest
// stack: (empty)
global validate_jumpdest_table:
    // If address > 0 it is interpreted as address' = address - 1
    // and the next prover input should contain a proof for address'.
    PROVER_INPUT(jumpdest_table::next_address)
    DUP1 %jumpi(check_proof)
    // If proof == 0 there are no more jump destionations to check
    POP
global validate_jumpdest_table_end:
    JUMP
    // were set to 0
    //%mload_context_metadata(@CTX_METADATA_CODE_SIZE)
    // get the code length in bytes
    //%add_const(31)
    //%div_const(32)
    //GET_CONTEXT
    //SWAP2
//verify_chunk:
    // stack: i (= proof), code_len, ctx = 0
    //%stack (i, code_len, ctx) -> (code_len, i, ctx, @SEGMENT_JUMPDEST_BITS, i, 32, i, code_len, ctx)
    //GT
    //%jumpi(valid_table)
    //%mload_packing
    // stack: packed_bits, code_len, i, ctx
    //%assert_eq_const(0)
    //%increment
    //%jump(verify_chunk)

check_proof:
    %sub_const(1)
    DUP1
    // We read the proof
    PROVER_INPUT(jumpdest_table::next_proof)
    // stack: proof, address
    %is_jumpdest
    GET_CONTEXT
    %stack (ctx, address) -> (1, ctx, @SEGMENT_JUMPDEST_BITS, address)
    MSTORE_GENERAL
    %jump(validate_jumpdest_table)
valid_table:
    // stack: ctx, @SEGMENT_JUMPDEST_BITS, i, 32, i, code_len, ctx, retdest
    %pop7
    JUMP

%macro validate_jumpdest_table
    PUSH %%after
    %jump(validate_jumpdest_table)
%%after:
%endmacro
