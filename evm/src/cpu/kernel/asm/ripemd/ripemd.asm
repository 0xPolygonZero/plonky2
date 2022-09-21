/// Variables beginning with _ are in memory
///
/// def ripemd160(_input):
///     state, count, _buffer = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0], 0, [0]*64
///     state, count, _buffer = ripemd_update(state, count, _buffer,           len(input) , bytes =          _input  )
///     state, count, _buffer = ripemd_update(state, count, _buffer, padlength(len(input)), bytes =     [0x80]+[0]*63)
///     state, count, _buffer = ripemd_update(state, count, _buffer,                     8, bytes = size(len(_input)))
///     return process(state)
///
/// ripemd is called on a stack with ADDR and length
/// ripemd_update will receive and return the stack in the form:
///     stack: STATE, count, length, virt
/// where virt is the virtual address of the bytes argument

global ripemd_alt:
    // stack: length, INPUT
    %stack (length) -> (64, length, 0x80, 63, length, length)
    // stack:           64, length, 0x80, 63, length, length, INPUT

    %jump(0xdeadbeef)
    %jump(ripemd_storage) // stores the following into memory
                          // init  _buffer  at virt 0   [consumes           64]
                          // store _size    at virt 64  [consumes       length]
                          // store _padding at virt 72  [consumes 0x80,     63]
                          // store _input   at virt 136 [consumes       length]

global ripemd:
    // stack:  ADDR, length
    %stack (ADDR: 3, length) -> (64, length, 0x80, 63, ADDR, length, length)
    // stack:                    64, length, 0x80, 63, ADDR, length, length
    %jump(ripemd_storage) // stores the following into memory
                          // init  _buffer  at virt 0   [consumes           64]
                          // store _size    at virt 64  [consumes       length]
                          // store _padding at virt 72  [consumes 0x80,     63]
                          // store _input   at virt 136 [consumes ADDR, length]

global ripemd_init:
    // stack: length
    %stack (length) -> (        0, length,        136, ripemd_1, ripemd_2, process)
    // stack:           count = 0, length, virt = 136, ripemd_1, ripemd_2, process
    %stack (ARGS: 3, LABELS: 3) -> (0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0, ARGS,                LABELS)
    // stack:                       0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0, count, length, virt, LABELS
    %jump(ripemd_update)
ripemd_1:
    // stack:                                  STATE, count, length            , virt     , LABELS
    DUP7
    // stack:                          length, STATE, count, length            , virt     , LABELS
    %padlength
    // stack:                       padlength, STATE, count, length            , virt     , LABELS
    SWAP7
    POP
    // stack:                                  STATE, count, length = padlength, virt     , LABELS
    %stack (STATE: 5, count, length, virt) -> (STATE, count, length,                    72)
    //                                         STATE, count, length            , virt = 72, LABELS
    %jump(ripemd_update)
ripemd_2:
    // stack:                                  STATE, count, length    , virt     , LABELS
    %stack (STATE: 5, count, length, virt) -> (STATE, count,          8,        64)
    // stack:                                  STATE, count, length = 8, virt = 64, LABELS
    %jump(ripemd_update)
process:
    // stack: a , b, c, d, e, count, length, virt
    %flip_bytes_u32
    // stack: a', b, c, d, e, *vars
    SWAP1
    %flip_bytes_u32
    %shl_const(32)
    OR
    // stack: b' a', c, d, e, *vars
    SWAP1
    %flip_bytes_u32
    %shl_const(64)
    OR
    // stack: c' b' a', d, e, *vars
    SWAP1
    %flip_bytes_u32
    %shl_const(96)
    OR 
    // stack: d' c' b' a', e, *vars
    SWAP1
    %flip_bytes_u32
    %shl_const(96)
    OR 
    // stack: e' d' c' b' a', *vars
    %stack (result, x, y, z) -> (result)
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
