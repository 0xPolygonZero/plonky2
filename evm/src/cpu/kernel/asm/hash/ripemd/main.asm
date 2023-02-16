/// Variables beginning with _ are in memory
///
/// def ripemd160(_input):
///     STATE, count, _buffer = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0], 0, [0]*64
///     STATE, count, _buffer = ripemd_update(STATE, count, _buffer,           len(input) , bytes =          _input  )
///     STATE, count, _buffer = ripemd_update(STATE, count, _buffer, padlength(len(input)), bytes =     [0x80]+[0]*63)
///     STATE, count, _buffer = ripemd_update(STATE, count, _buffer,                     8, bytes = size(len(_input)))
///     return process(STATE)
///
/// ripemd is called on
///     // stack: length
///
/// ripemd_update receives and return the stack in the form:
///     stack: STATE, count, length, virt
/// where virt is the virtual address of the bytes argument

global ripemd:
    // stack:                               virt, length
    %stack (virt, length) -> (length, 0x80, virt, length)
    // stack:                 length, 0x80, virt, length

    // stack: length
    %shl_const(3)
    // stack: abcdefgh
    %extract_and_store_byte(64)
    // stack: abcdefg
    %extract_and_store_byte(65)
    // stack: abcdef
    %extract_and_store_byte(66)
    // stack: abcde
    %extract_and_store_byte(67)
    // stack: abcd
    %extract_and_store_byte(68)
    // stack: abc
    %extract_and_store_byte(69)
    // stack: ab
    %extract_and_store_byte(70)
    // stack: a
    %mstore_kernel_general(71)

    // stack: 0x80
    %mstore_kernel_general(72)

    // stack: virt, length
    %stack (virt, length) -> (        0, length, virt, ripemd_1, ripemd_2, process)
    // stack:                 count = 0, length, virt, ripemd_1, ripemd_2, process
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


%macro extract_and_store_byte(offset)
    // stack: xsy
    PUSH 0x100
    DUP2
    MOD
    // stack: y, xsy
    %stack (y, xsy) -> (xsy, y, 0x100, y)
    // stack:           xsy, y, 0x100, y
    SUB
    DIV
    SWAP1
    // stack: y, xs
    %mstore_kernel_general($offset)
    // stack: xs
%endmacro 
