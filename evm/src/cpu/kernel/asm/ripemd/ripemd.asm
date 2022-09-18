/// Variables beginning with _ are in memory
///
/// def ripemd160(_input):
///     state, count, _buffer = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0], 0, [0]*64
///     state, count, _buffer = ripemd_update(state, count, _buffer,           len(input) ,          _input  )
///     state, count, _buffer = ripemd_update(state, count, _buffer, padlength(len(input)),     [0x80]+[0]*63)
///     state, count, _buffer = ripemd_update(state, count, _buffer,                     8, size(len(_input)))
///     return process(state)

/// ripemd is called on a stack with ADDR and length
/// ripemd_update will receive and return the stack in the form:
///     stack: *state, count, length, offset
/// where offset is the virtual address of its final positional argument 

global ripemd:
    // stack:         ADDR, length
    DUP4
    // stack: length, ADDR, length
    %init_buffer    // init  _buffer  at offset 0
    %store_size     // store _size    at offset 64  [consumes length]
    %store_padding  // store _padding at offset 72
    %store_input    // store _input   at offset 136 [consumes ADDR]
    // stack: length
    %stack (length) -> (        0, length,          136, ripemd_1, ripemd_2, process)
    // stack:           count = 0, length, offset = 136, ripemd_1, ripemd_2, process
    %stack (c, l, o, l1, l2, l3) -> (0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0,     c,      l,      o, l1, l2, l3)
    // stack:                        0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0, count, length, offset, *labels
    %jump(ripemd_update)
ripemd_1:
    // stack:            *state, count, length            , offset, *labels
    DUP7
    // stack:    length, *state, count, length            , offset, *labels
    %padlength
    // stack: padlength, *state, count, length            , offset, *labels
    SWAP7
    POP
    // stack:            *state, count, length = padlength, offset, *labels
    %stack (a, b, c, d, e, count, length, offset) -> (a, b, c, d, e, count, length, 72)
    %jump(ripemd_update)
ripemd_2:
    // stack:            *state, count, length, offset, *labels
    %stack (a, b, c, d, e, count, length, offset) -> (a, b, c, d, e, count, 8, 64)
    // stack:            *state, count, length, offset, *labels
    %jump(ripemd_update)
process:
    // stack: a , b, c, d, e, count, length, offset
    %flip_bytes_u32
    // stack: a', b, c, d, e, *vars
    SWAP1
    %flip_bytes_32
    %shl_const(32)
    OR
    // stack: b' a', c, d, e, *vars
    SWAP1
    %flip_bytes_32
    %shl_const(64)
    OR
    // stack: c' b' a', d, e, *vars
    SWAP1
    %flip_bytes_32
    %shl_const(96)
    OR 
    // stack: d' c' b' a', e, *vars
    SWAP1
    %flip_bytes_32
    %shl_const(96)
    OR 
    // stack: e' d' c' b' a', *vars
    %stack (result, x, y, z) -> result
    // stack: result


/// def padlength(length):
///    x = 56 - length % 64
///    return x + 64*(x < 9)

%macro padlength
    // stack:          count
    %mod_const(64)
    // stack:          count % 64
    PUSH 56
    SUB
    // stack: x = 56 - count % 64
    DUP1
    %lt_const(9)
    // stack:     x < 9  , x
    %mul_const(64)
    // stack: 64*(x < 9) , x
    ADD
    // stack: 64*(x < 9) + x
%endmacro
