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
    %stack (JUMPDEST, i, ctx) -> (ctx, @SEGMENT_JUMPDEST_BITS, i, 1, JUMPDEST, i, ctx)
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

// Determines how many bytes to skip, if any, based on the opcode we read.
// If we read a PUSH<n> opcode, we skip over n bytes, otherwise we skip 0.
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
