global ripemd:
    JUMPDEST
    // stack: retdest
    PUSH 0xC3D2E1F0
    PUSH 0x10325476
    PUSH 0x98BADCFE
    PUSH 0xEFCDAB89
    PUSH 0x67452301
    // stack: 0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0, retdest

process:
    JUMPDEST
    // stack: a , b, c, d, e, count, retdest
    %flip_bytes_u32
    // stack: a', b, c, d, e, count, retdest
    SWAP1
    %flip_bytes_32
    %shl_const(32)
    OR
    // stack: b' a', c, d, e, count, retdest
    SWAP1
    %flip_bytes_32
    %shl_const(64)
    OR
    // stack: c' b' a', d, e, count, retdest
    SWAP1
    %flip_bytes_32
    %shl_const(96)
    OR 
    // stack: d' c' b' a', e, count, retdest
    SWAP1
    %flip_bytes_32
    %shl_const(96)
    OR 
    // stack: e' d' c' b' a', count, retdest
    SWAP2
    SWAP1
    POP
    // stack: retdest, e'd'c'b'a'
    JUMP
