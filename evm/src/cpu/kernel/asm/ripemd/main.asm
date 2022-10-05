/// Variables beginning with _ are in memory
///
/// def ripemd160(_input):
///     STATE, count, _buffer = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0], 0, [0]*64
///     STATE, count, _buffer = ripemd_update(STATE, count, _buffer,           len(input) , bytes =          _input  )
///     STATE, count, _buffer = ripemd_update(STATE, count, _buffer, padlength(len(input)), bytes =     [0x80]+[0]*63)
///     STATE, count, _buffer = ripemd_update(STATE, count, _buffer,                     8, bytes = size(len(_input)))
///     return process(STATE)
///
/// ripemd is called on a stack with ADDR and length
/// ripemd_stack is called on a stack with length, followed by the input bytes
///
/// ripemd_update receives and return the stack in the form:
///     stack: STATE, count, length, virt
/// where virt is the virtual address of the bytes argument

global ripemd_stack:
    // stack: length, INPUT
    %stack (length) -> (64, length, 0x80, 63, length, length)
    // stack:           64, length, 0x80, 63, length, length, INPUT
    %jump(ripemd_storage) // stores the following into memory
                          // init  _buffer  at virt 0   [consumes           64]
                          // store _size    at virt 64  [consumes       length]
                          // store _padding at virt 72  [consumes 0x80,     63]
                          // store _input   at virt 136 [consumes       length]

global ripemd:
    // stack:  ADDR, length
    %stack (ADDR: 3, length) -> (64, length, 0x80, 63, length, ADDR, length)
    // stack:                    64, length, 0x80, 63, length, ADDR, length
    %jump(ripemd_storage) // stores the following into memory
                          // init  _buffer  at virt 0   [consumes           64]
                          // store _size    at virt 64  [consumes       length]
                          // store _padding at virt 72  [consumes 0x80,     63]
                          // store _input   at virt 136 [consumes ADDR, length]

global ripemd_init:
    // stack: length
    %stack (length) -> (        0, length,        136, ripemd_1, ripemd_2, process)
    // stack:           count = 0, length, virt = 136, ripemd_1, ripemd_2, process
    %stack () -> (0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0)
    // stack:     0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0, count, length, virt, LABELS
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
global process:
    // stack: a , b, c, d, e, count, length, virt
    %reverse_bytes_u32
    %shl_const(128)
    // stack: a', b, c, d, e, VARS
    SWAP1
    %reverse_bytes_u32
    %shl_const(96)
    OR
    // stack: b' a', c, d, e, VARS
    SWAP1
    %reverse_bytes_u32
    %shl_const(64)
    OR
    // stack: c' b' a', d, e, VARS
    SWAP1
    %reverse_bytes_u32
    %shl_const(32)
    OR 
    // stack: d' c' b' a', e, VARS
    SWAP1
    %reverse_bytes_u32
    OR 
    // stack: e' d' c' b' a', VARS
    %stack (result, VARS: 3, retdest) -> (retdest, result)
    // stack: 0xdeadbeef, result
    JUMP


/// def padlength(length):
///     t = length % 64
///     return 56 + 64*(t > 55) - t

%macro padlength
    // stack:          count
    %mod_const(64)
    // stack:      t = count % 64
    PUSH 55
    DUP2
    // stack:          t , 55 , t
    GT
    // stack:          t > 55 , t
    %mul_const(64)
    %add_const(56)
    // stack: 56 + 64*(t > 55), t 
    SUB
%endmacro
