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
///     R = length - shift > 0
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
    %stack (shift, need, have, STATE: 5, count, length) -> (length, need, STATE, 0, shift, need, have, count, length)
    // stack:                                               length, need, STATE, 0, shift, need, have, count, length, virt, retdest
    LT 
    NOT
    // stack:               Q, STATE, 0, shift, need, have, count, length, virt, retdest
    %stack (Q, STATE: 5, i, shift, need, have) -> (have, Q, Q, STATE, i, shift, need, have) 
    AND
    // stack:            P, Q, STATE, 0, shift, need, have, count, length, virt, retdest
    %jumpi(update_1)
    // stack:               Q, STATE, 0, shift, need, have, count, length, virt, retdest
    %jumpi(update_2)
final_update:
    // stack:          shift, need, have, STATE, count, length, virt, retdest
    %stack (shift, need, have, STATE: 5, count, length) -> (length, shift, return_step, shift, need, have, STATE, count, length)
    SUB
    // stack:                                                                     ARGS, shift, need, have, STATE, count, length, virt, retdest
    %stack (ARGS: 2, shift, need, have, STATE: 5, count, length, virt) -> (shift, virt, have, ARGS: 2, shift, need, have, STATE, count, length, virt)
    ADD
    // stack:                                                                  ARGS: 4, shift, need, have, STATE, count, length, virt, retdest
    PUSH 0
    DUP4
    GT
    // stack:                                                                  R, ARGS, shift, need, have, STATE, count, length, virt, retdest
    %jumpi(buffer_update)
    // stack:                                                                     ARGS, shift, need, have, STATE, count, length, virt, retdest
    %pop3
    JUMP
return_step:
    // stack:          shift, need, have, STATE, count, length, virt, retdest
    SWAP8
    DUP10
    %mul_const(8)
    ADD
    SWAP8
    // stack:          shift, need, have, STATE, count += 8*length, length, virt, retdest
    %stack (shift, need, have, STATE: 5, count, length, virt, retdest) -> (retdest, STATE, count, length, virt)
    JUMP


/// def update_1():
///     buffer_update(virt, have, need)
///     shift = need
///     have  = 0
///     state = compress(state, buffer)

update_1:
    // stack: Q, STATE, 0, shift, need, have, count, length, virt, retdest
    %stack (Q, STATE: 5, i, shift, need, have, count, length, virt) -> (virt, have, need, update_1a, STATE, i, shift, need, have, count, length, virt)
    %jump(buffer_update)
update_1a:
    // stack: STATE, 0, shift, need, have, count, length, virt, retdest
    %stack (STATE: 5, i, shift, need, have) -> (STATE, i, update_2,         need, need,        0)
    // stack:                                   STATE, 0, update_2, shift = need, need, have = 0, count, length, virt, retdest
    %jump(compress)

/// def update_2():
///     cond = length - shift - 64
///     while cond >= 0:
///         state   = compress(state, bytestring[shift:])
///         shift += 64
///         cond  -= 64

update_2:
    // stack:               STATE, shift, need, have, count, length, virt, retdest
    %stack (STATE: 5, shift, need, have, count, length) -> (length, shift, STATE, shift, need, have, count, length) 
    SUB
    SUB
    // stack:         cond, STATE, shift, need, have, count, length, virt, retdest
    DUP12
    DUP8
    ADD
    // stack: offset, cond,  STATE, shift, need, have, count, length, virt, retdest
    %stack (offset, cond, STATE: 5) -> (cond, 0, STATE, offset, compression_loop, cond)
    LT
    NOT
    // cond >= 0, STATE, offset, compression_loop, cond, shift, need, have, count, length, virt, retdest
    %jumpi(compress)
compression_loop:
    // stack: STATE, offset   ,        cond   , shift, need, have, count, length, virt, retdest
    SWAP5
    %add_const(64)
    SWAP5 
    SWAP6
    %sub_const(64)
    SWAP6
    SWAP7 
    %add_const(64)
    SWAP7 
    // stack: STATE, offset+64,        cond-64, shift+64, need, have, count, length, virt, retdest
    %stack (STATE: 5, offset, cond, shift) -> (cond, 0, STATE, offset, compression_loop, cond, shift)
    %jumpi(compress)
    // stack: STATE, offset   , label, cond   , shift   , need, have, count, length, virt, retdest
    %stack (STATE: 5, offset, label, cond, shift, need, have, count, length, virt, retdest) -> (shift, need, have, STATE, count, length, virt, retdest)
    %jump(final_update)


/// def buffer_update(get, set, times):
///     for i in range(times):
///         buffer[set+i] = bytestring[get+i]

buffer_update: 
    // stack:           get  , set  , times  , retdest
    DUP2
    DUP2
    // stack: get, set, get  , set  , times  , retdest
    %mupdate_ripemd
    // stack:           get  , set  , times  , retdest
    %add_const(1)
    SWAP1 
    %add_const(1)
    SWAP1
    SWAP2
    %sub_const(1)
    SWAP2
    // stack:           get+1, set+1, times-1, retdest
    DUP3
    %jumpi(buffer_update)
    // stack:           get  , set  , 0      , retdest
    %pop3
    JUMP
