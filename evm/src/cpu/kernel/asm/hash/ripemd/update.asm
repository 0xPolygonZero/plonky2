/// ripemd_update will receive and return the stack in the form:
///     stack: STATE, count, length, virt
///
/// def ripemd_update(state, count, buffer, length, bytestring):
///     have  = (count // 8) % 64
///     need  = 64 - have
///     shift = 0
///     P = length >= need and have
///     Q = length >= need
///     if P: 
///         update_1()
///     if Q:
///         update_2()
///     R = length > shift
///     if R:
///         buffer_update(virt + shift, have, length - shift)
/// 
///     return state, count + 8*length, buffer

global ripemd_update:
    // stack:                           STATE, count, length, virt, retdest
    %stack (STATE: 5, count, length, virt) -> (count, 8, 64, STATE, count, length, virt)
    DIV
    MOD
    // stack:                     have, STATE, count, length, virt, retdest
    DUP1
    PUSH 64
    SUB
    PUSH 0
    // stack:        shift, need, have, STATE, count, length, virt, retdest
    %stack (shift, need, have, STATE: 5, count, length) -> (length, need, STATE, shift, need, have, count, length)
    // stack:                                               length, need, STATE, shift, need, have, count, length, virt, retdest
    LT 
    ISZERO
    // stack:               Q, STATE, shift, need, have, count, length, virt, retdest
    %stack (Q, STATE: 5, shift, need, have) -> (have, Q, Q, STATE, shift, need, have)
    %gt_const(0)
    AND
    // stack:            P, Q, STATE, shift, need, have, count, length, virt, retdest
    %jumpi(update_1)
    // stack:               Q, STATE, shift, need, have, count, length, virt, retdest
    %jumpi(update_2)
final_update:
    // stack:                                                                           STATE, shift, need, have, count, length, virt, retdest
    %stack (STATE: 5, shift, need, have, count, length) -> (length, shift, return_step, STATE, shift, need, have, count, length)
    SUB
    // stack:                                                                  ARGS: 2, STATE, shift, need, have, count, length, virt, retdest
    %stack (ARGS: 2, STATE: 5, shift, need, have, count, length, virt) -> (shift, virt, have, ARGS, STATE, shift, need, have, count, length, virt)
    ADD
    // stack:                                                                  ARGS: 4, STATE, shift, need, have, count, length, virt, retdest
    %stack (ARGS: 4, STATE: 5, shift, need, have, count, length) -> (length, shift, ARGS, STATE, shift, need, have, count, length)
    GT
    // stack:                                                               R, ARGS: 4, STATE, shift, need, have, count, length, virt, retdest
    %jumpi(buffer_update)
    // stack:                                                                  ARGS: 4, STATE, shift, need, have, count, length, virt, retdest
    %pop3
    JUMP
return_step:
    // stack:          STATE, shift, need, have, count, length, virt, retdest
    SWAP8
    DUP10
    %mul_const(8)
    ADD
    SWAP8
    // stack:          STATE, shift, need, have, count, length, virt, retdest
    %stack (STATE: 5, shift, need, have, count, length, virt, retdest) -> (retdest, STATE, count, length, virt)
    JUMP


/// def update_1():
///     buffer_update(virt, have, need)
///     shift = need
///     have  = 0
///     state = compress(state, buffer)

update_1:
    // stack: Q, STATE, shift, need, have, count, length, virt, retdest
    %stack (Q, STATE: 5, shift, need, have, count, length, virt) -> (virt, have, need, update_1a, STATE, shift, need, have, count, length, virt)
    %jump(buffer_update)
update_1a:
    // stack:    STATE, shift, need, have, count, length, virt, retdest
    %stack (STATE: 5, shift, need, have) -> (STATE, 0, update_2,         need, need,        0)
    // stack:                                STATE, 0, update_2, shift = need, need, have = 0, count, length, virt, retdest
    %jump(compress)

/// def update_2():
///     while length >= shift + 64:
///         shift += 64
///         state  = compress(state, bytestring[shift-64:])

update_2:
    // stack:       STATE, shift, need, have, count, length, virt, retdest
    %stack (STATE: 5, shift, need, have, count, length) -> (64, shift, length, STATE, shift, need, have, count, length) 
    ADD
    GT
    // stack: cond, STATE, shift, need, have, count, length, virt, retdest
    %jumpi(final_update)
    SWAP5
    %add_const(64)
    SWAP5
    %stack (STATE: 5, shift) -> (shift, 64, STATE, shift)
    DUP13
    ADD
    SUB
    // stack: offset, STATE, shift, need, have, count, length, virt, retdest
    %stack (offset, STATE: 5) -> (STATE, offset, update_2)
    // stack: STATE, offset, update_2, shift, need, have, count, length, virt, retdest
    %jump(compress)
