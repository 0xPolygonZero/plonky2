/// Variables beginning with _ are in memory and not on the stack
/// ripemd_update will receive and return the stack in the form:
///     stack: *state, count, length, offset
/// where offset is the virtual address of its final positional argument 
///
/// def ripemd160(_input):
///     state, count, _buffer = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0], 0, [0]*64
///     state, count, _buffer = ripemd_update(state, count, _buffer,      len(_input),   _input)
///     _padding = [0x80]+[0]*63
///     _size    = get_size(count)
///     state, count, _buffer = ripemd_update(state, count, _buffer, padlength(count), _padding)
///     state, count, _buffer = ripemd_update(state, count, _buffer,                8,    _size)
///     return process(state)

global ripemd:
    // stack: ADDR, length
    %store_buffer   // store _buffer at location 0
    %store_input    // store _input  at location 64
    // stack: length
    %stack (length) -> (        0, length,          64, ripemd_1, ripemd_2, process)
    // stack:           count = 0, length, offset = 64, ripemd_1, ripemd_2, process
    %stack (c, l, o, l1, l2, l3) -> (0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0,     c,      l,      o, l1, l2, l3)
    // stack:                        0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0, count, length, offset, *labels
    %jump(ripemd_update)
ripemd_1:
    // stack:                   *state, count, length, offset, *labels
    DUP6
    DUP1
    // stack:     count, count, *state, count, length, offset, *labels
    %store_padding  // store _padding at location 64
    %store_size     // store _size    at location 128 [note: consumes count]
    %padlength
    // stack: padlength, *state, count, length, offset, *labels
    SWAP7
    POP
    // stack:            *state, count, length, offset, *labels
    %jump(ripemd_update)
ripemd_2:
    // stack:            *state, count, length, offset, *labels
    %stack (a, b, c, d, e, count, length, offset) -> (a, b, c, d, e, count, 8, 128)
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


/// def padlength(count):
///    x = 56 - (count // 8) % 64
///    return x + 64*(x < 9)

%macro padlength
    // stack:           count
    %div_const(8)
    // stack:           count // 8
    %mod_const(64)
    // stack:          (count // 8) % 64
    PUSH 56
    SUB
    // stack: x = 56 - (count // 8) % 64
    DUP1
    %lt_const(9)
    // stack:     x < 9  , x
    %mul_const(64)
    // stack: 64*(x < 9) , x
    ADD
    // stack: 64*(x < 9) + x
%endmacro
