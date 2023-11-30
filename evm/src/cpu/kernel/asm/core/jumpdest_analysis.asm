// Populates @SEGMENT_JUMPDEST_BITS for the given context's code.
// Pre stack: ctx, code_len, retdest
// Post stack: (empty)
global jumpdest_analysis:
    // stack: ctx, code_len, retdest
    PUSH 0 // i = 0
    %stack (i, ctx, code_len, retdest) -> (i, ctx, code_len, retdest, 0) // ctr

global loop:
    // stack: i, ctx, code_len, retdest
    // Ideally we would break if i >= code_len, but checking i > code_len is
    // cheaper. It doesn't hurt to over-read by 1, since we'll read 0 which is
    // a no-op.
    DUP3 DUP2 GT // i > code_len
    %jumpi(return)

    %stack (i, ctx) -> (ctx, @SEGMENT_CODE, i, 32, i, ctx)
    %mload_packing
    // stack: packed_opcodes
    DUP1
    PUSH 0x8080808080808080808080808080808080808080808080808080808080808080
    AND
global debug_before_as_dad:
    %jumpi(as_dad)
global debug_wuau:
as_dad:
    POP
global debug_not_wuau:


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
    %stack (opcode, i, ctx, code_len, retdest, ctr) -> (ctr, opcode, i, ctx, code_len, retdest)
    %increment
    %stack (ctr, opcode, i, ctx, code_len, retdest) -> (opcode, i, ctx, code_len, retdest, ctr)

global continue:
    // stack: opcode, i, ctx, code_len, retdest
    %add_const(code_bytes_to_skip)
    %mload_kernel_code
    // stack: bytes_to_skip, i, ctx, code_len, retdest
    ADD
    // stack: i, ctx, code_len, retdest
    %jump(loop)

global return:
    // stack: i, ctx, code_len, retdest
    %pop3
    SWAP1
global debug_ctr:
    POP
    JUMP

// Determines how many bytes away is the next opcode, based on the opcode we read.
// If we read a PUSH<n> opcode, next opcode is in n + 1 bytes, otherwise it's the next one.
//
// Note that the range of PUSH opcodes is [0x60, 0x80). I.e. PUSH1 is 0x60
// and PUSH32 is 0x7f.
global code_bytes_to_skip:
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
