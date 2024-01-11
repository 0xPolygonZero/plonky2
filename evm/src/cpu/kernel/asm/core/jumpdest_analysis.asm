// Set @SEGMENT_JUMPDEST_BITS to one between positions [init_pos, final_pos], 
// for the given context's code.
// Pre stack: init_pos, ctx, final_pos, retdest
// Post stack: (empty)
global verify_path_and_write_jumpdest_table:
loop:
    // stack: i, ctx, final_pos, retdest
    DUP3 DUP2 EQ // i == final_pos
    %jumpi(proof_ok)
    DUP3 DUP2 GT // i > final_pos
    %jumpi(proof_not_ok)

     // stack: i, ctx, final_pos, retdest
    %stack (i, ctx) -> (ctx, i, i, ctx)
    ADD // combine context and offset to make an address (SEGMENT_CODE == 0)
    MLOAD_GENERAL
    // stack: opcode, i, ctx, final_pos, retdest

    DUP1 
    // Slightly more efficient than `%eq_const(0x5b) ISZERO`
    PUSH 0x5b
    SUB
    // stack: opcode != JUMPDEST, opcode, i, ctx, final_pos, retdest
    %jumpi(continue)

    // stack: JUMPDEST, i, ctx, code_len, retdest
    %stack (JUMPDEST, i, ctx) -> (ctx, @SEGMENT_JUMPDEST_BITS, i, JUMPDEST, i, ctx)
    %build_address
    PUSH 1
    // stack: 1, addr, JUMPDEST, i, ctx
    MSTORE_GENERAL

continue:
    // stack: opcode, i, ctx, final_pos, retdest
    %add_const(code_bytes_to_skip)
    %mload_kernel_code
    // stack: bytes_to_skip, i, ctx, final_pos, retdest
    ADD
    // stack: i, ctx, final_pos, retdest
    %jump(loop)

proof_ok:
    // stack: i, ctx, final_pos, retdest
    // We already know final_pos is a jumpdest
    %stack (i, ctx, final_pos) -> (ctx, @SEGMENT_JUMPDEST_BITS, i)
    %build_address
    PUSH 1
    MSTORE_GENERAL
    JUMP
proof_not_ok:
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


// A proof attesting that jumpdest is a valid jump destination is
// either 0 or an index 0 < i <= jumpdest - 32.
// A proof is valid if:
// - i == 0 and we can go from the first opcode to jumpdest and code[jumpdest] = 0x5b
// - i > 0 and:
//     a) for j in {i+0,..., i+31} code[j] != PUSHk for all k >= 32 - j - i,
//     b) we can go from opcode i+32 to jumpdest,
//     c) code[jumpdest] = 0x5b.
// To reduce the number of instructions, when i > 32 we load all the bytes code[j], ...,
// code[j + 31] in a single 32-byte word, and check a) directly on the packed bytes.
// We perform the "packed verification" computing a boolean formula evaluated on the bits of 
// code[j],..., code[j+31] of the form p_1 AND p_2 AND p_3 AND p_4 AND p_5, where:
//     - p_k is either TRUE, for one subset of the j's which depends on k (for example,
//       for k = 1, it is TRUE for the first 15 positions), or has_prefix_k => bit_{k + 1}_is_0
//       for the j's not in the subset.
//     - has_prefix_k is a predicate that is TRUE if and only if code[j] has the same prefix of size k + 2
//       as PUSH{32-(j-i)}.
// stack: proof_prefix_addr, jumpdest, ctx, retdest
// stack: (empty)
global write_table_if_jumpdest:
    // stack: proof_prefix_addr, jumpdest, ctx, retdest
    %stack
        (proof_prefix_addr, jumpdest, ctx) ->
        (ctx, jumpdest, jumpdest, ctx, proof_prefix_addr)
    ADD // combine context and offset to make an address (SEGMENT_CODE == 0)
    MLOAD_GENERAL
    // stack: opcode, jumpdest, ctx, proof_prefix_addr, retdest

    %jump_neq_const(0x5b, return)

    //stack: jumpdest, ctx, proof_prefix_addr, retdest
    SWAP2 DUP1
    // stack: proof_prefix_addr, proof_prefix_addr, ctx, jumpdest
    ISZERO
    %jumpi(verify_path_and_write_jumpdest_table)


    // stack: proof_prefix_addr, ctx, jumpdest, retdest
    // If we are here we need to check that the next 32 bytes are less
    // than JUMPXX for XX < 32 - i <=> opcode < 0x7f - i = 127 - i, 0 <= i < 32,
    // or larger than 127
    
    %stack
        (proof_prefix_addr, ctx) ->
        (ctx, proof_prefix_addr, 32, proof_prefix_addr, ctx)
    ADD // combine context and offset to make an address (SEGMENT_CODE == 0)
    %mload_packing
    // packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    DUP1 %shl_const(1)
    DUP2 %shl_const(2)
    AND
    // stack: (is_1_at_pos_2_and_3|(X)⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    // X denotes any value in {0,1} and Z^i is Z repeated i times
    NOT
    // stack: (is_0_at_2_or_3|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    DUP2
    OR
    // stack: (is_1_at_1 or is_0_at_2_or_3|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    // stack: (~has_prefix|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest

    // Compute in_range = 
    //   - (0xFF|X⁷)³² for the first 15 bytes
    //   - (has_prefix => is_0_at_4 |X⁷)³² for the next 15 bytes
    //   - (~has_prefix|X⁷)³² for the last byte
    // Compute also ~has_prefix = ~has_prefix OR is_0_at_4 for all bytes. We don't need to update ~has_prefix
    // for the second half but it takes less cycles if we do it.
    DUP2 %shl_const(3)
    NOT
    // stack: (is_0_at_4|X⁷)³²,  (~has_prefix|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    // pos 0102030405060708091011121314151617181920212223242526272829303132
    PUSH 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00
    AND
    // stack: (is_0_at_4|X⁷)³¹|0⁸,  (~has_prefix|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    DUP2
    DUP2
    OR
    // pos 0102030405060708091011121314151617181920212223242526272829303132
    PUSH 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF0000000000000000000000000000000000
    OR
    // stack: (in_range|X⁷)³², (is_0_at_4|X⁷)³²,  (~has_prefix|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    SWAP2
    OR
    // stack: (~has_prefix|X⁷)³², (in_range|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest

    // Compute in_range' = in_range AND
    //   - (0xFF|X⁷)³² for bytes in positions 1-7 and 16-23 
    //   - (has_prefix => is_0_at_5 |X⁷)³² on the rest
    // Compute also ~has_prefix = ~has_prefix OR is_0_at_5 for all bytes.

    DUP3 %shl_const(4)
    NOT
    // stack: (is_0_at_5|X⁷)³²,  (~has_prefix|X⁷)³², (in_range|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    DUP2
    DUP2
    OR
    // pos 0102030405060708091011121314151617181920212223242526272829303132
    PUSH 0xFFFFFFFFFFFFFF0000000000000000FFFFFFFFFFFFFFFF000000000000000000
    OR
    // stack: (in_range'|X⁷)³², (is_0_at_5|X⁷)³²,  (~has_prefix|X⁷)³², (in_range|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    SWAP2
    OR
    // stack: (~has_prefix|X⁷)³², (in_range'|X⁷)³², (in_range|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    SWAP2
    AND
    SWAP1 

    // Compute in_range' = in_range AND
    //   - (0xFF|X⁷)³² for bytes in positions 1-3, 8-11, 16-19, and 24-27 
    //   - (has_prefix => is_0_at_6 |X⁷)³² on the rest
    // Compute also that ~has_prefix = ~has_prefix OR is_0_at_4 for all bytes.

    // stack: (~has_prefix|X⁷)³², (in_range|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    DUP3 %shl_const(5)
    NOT
    // stack: (is_0_at_6|X⁷)³²,  (~has_prefix|X⁷)³², (in_range|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    DUP2
    DUP2
    OR
    // pos 0102030405060708091011121314151617181920212223242526272829303132
    PUSH 0xFFFFFF00000000FFFFFFFF00000000FFFFFFFF00000000FFFFFFFF0000000000
    OR
    // stack: (in_range'|X⁷)³², (is_0_at_6|X⁷)³²,  (~has_prefix|X⁷)³², (in_range|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    SWAP2
    OR
    // stack: (~has_prefix|X⁷)³², (in_range'|X⁷)³², (in_range|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    SWAP2
    AND
    SWAP1 

    // Compute in_range' = in_range AND
    //   - (0xFF|X⁷)³² for bytes in 1, 4-5, 8-9, 12-13, 16-17, 20-21, 24-25, 28-29
    //   - (has_prefix => is_0_at_7 |X⁷)³² on the rest
    // Compute also that ~has_prefix = ~has_prefix OR is_0_at_7 for all bytes.

    // stack: (~has_prefix|X⁷)³², (in_range|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    DUP3 %shl_const(6)
    NOT
    // stack: (is_0_at_7|X⁷)³²,  (~has_prefix|X⁷)³², (in_range|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    DUP2
    DUP2
    OR
    // pos 0102030405060708091011121314151617181920212223242526272829303132
    PUSH 0xFF0000FFFF0000FFFF0000FFFF0000FFFF0000FFFF0000FFFF0000FFFF000000
    OR
    // stack: (in_range'|X⁷)³², (is_0_at_7|X⁷)³²,  (~has_prefix|X⁷)³², (in_range|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    SWAP2
    OR
    // stack: (~has_prefix|X⁷)³², (in_range'|X⁷)³², (in_range|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    SWAP2
    AND
    SWAP1

    // Compute in_range' = in_range AND
    //   - (0xFF|X⁷)³² for bytes in odd positions
    //   - (has_prefix => is_0_at_8 |X⁷)³² on the rest

    // stack: (~has_prefix|X⁷)³², (in_range|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    DUP3 %shl_const(7)
    NOT
    // stack: (is_0_at_8|X⁷)³²,  (~has_prefix|X⁷)³², (in_range|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest
    OR
    // pos 0102030405060708091011121314151617181920212223242526272829303132
    PUSH 0x00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF00FF
    OR
    AND
    // stack: (in_range|X⁷)³², packed_opcodes, proof_prefix_addr, ctx, jumpdest, retdest

    // Get rid of the irrelevant bits
    // pos 0102030405060708091011121314151617181920212223242526272829303132
    PUSH 0x8080808080808080808080808080808080808080808080808080808080808080
    AND
    %jump_neq_const(0x8080808080808080808080808080808080808080808080808080808080808080, return)
    POP
    %add_const(32)

    // check the remaining path
    %jump(verify_path_and_write_jumpdest_table)
return:
    // stack: proof_prefix_addr, jumpdest, ctx, retdest
    %pop3
    JUMP

%macro write_table_if_jumpdest
    %stack (proof_prefix_addr, jumpdest, ctx) -> (proof_prefix_addr, jumpdest, ctx, %%after)
    %jump(write_table_if_jumpdest)
%%after:
%endmacro

// Write the jumpdest table. This is done by
// non-deterministically guessing the sequence of jumpdest
// addresses used during program execution within the current context.
// For each jumpdest address we also non-deterministically guess
// a proof, which is another address in the code such that 
// is_jumpdest doesn't abort, when the proof is at the top of the stack
// an the jumpdest address below. If that's the case we set the
// corresponding bit in @SEGMENT_JUMPDEST_BITS to 1.
// 
// stack: ctx, code_len, retdest
// stack: (empty)
global jumpdest_analysis:
    // If address > 0 then address is interpreted as address' + 1
    // and the next prover input should contain a proof for address'.
    PROVER_INPUT(jumpdest_table::next_address)
    DUP1 %jumpi(check_proof)
    // If address == 0 there are no more jump destinations to check
    POP
// This is just a hook used for avoiding verification of the jumpdest
// table in another context. It is useful during proof generation,
// allowing the avoidance of table verification when simulating user code.
global jumpdest_analysis_end:
    %pop2
    JUMP
check_proof:
    // stack: address, ctx, code_len, retdest
    DUP3 DUP2 %assert_le
    %decrement
    // stack: proof, ctx, code_len, retdest
    DUP2 SWAP1
    // stack: address, ctx, ctx, code_len, retdest
    // We read the proof
    PROVER_INPUT(jumpdest_table::next_proof)
    // stack: proof, address, ctx, ctx, code_len, retdest
    %write_table_if_jumpdest
    // stack: ctx, code_len, retdest
    
    %jump(jumpdest_analysis)

%macro jumpdest_analysis
    %stack (ctx, code_len) -> (ctx, code_len, %%after)
    %jump(jumpdest_analysis)
%%after:
%endmacro
