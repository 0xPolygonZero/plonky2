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

    DUP1 %eq_const(0x5b)
    // stack: opcode == JUMPDEST, opcode, i, ctx, code_len, retdest
    %jumpi(encountered_jumpdest)

    // stack: opcode, i, ctx, code_len, retdest
    %code_bytes_to_skip
    // stack: bytes_to_skip, i, ctx, code_len, retdest
    ADD
    %jump(continue)

encountered_jumpdest:
    // stack: opcode, i, ctx, code_len, retdest
    POP
    // stack: i, ctx, code_len, retdest
    %stack (i, ctx) -> (ctx, @SEGMENT_JUMPDEST_BITS, i, 1, i, ctx)
    MSTORE_GENERAL

continue:
    // stack: i, ctx, code_len, retdest
    %increment
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
%macro code_bytes_to_skip
    // stack: opcode
    %sub_const(0x60)
    // stack: opcode - 0x60
    DUP1 %lt_const(0x20)
    // stack: is_push_opcode, opcode - 0x60
    SWAP1
    %increment // n = opcode - 0x60 + 1
    // stack: n, is_push_opcode
    MUL
    // stack: bytes_to_skip
%endmacro
