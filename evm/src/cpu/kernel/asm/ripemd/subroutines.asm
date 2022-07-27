global Rol:
    jumpdest
    // stack: 2, 1
    dup2
    // stack: 2, 1, 1
    dup1
    // stack: 2, 2, 1, 1
    swap2
    // stack: 1, 2, 2, 1
    push 32
    // stack: 32, 1, 2, 2, 1
    sub
    // stack: 31, 2, 2, 1
    swap1
    // stack: 2, 31, 2, 1
    shr
    // stack: 0, 2, 1
    swap2
    // stack: 1, 2, 0
    swap1
    // stack: 2, 1, 0
    shl
    // stack: 4, 0
    push 4294967295
    // stack: 4294967295, 4, 0
    and
    // stack: 4, 0
    or
    // stack: 4
    jump

global F0:
    jumpdest
    // stack: x, y, z
    xor
    // stack: x ^ y, z
    xor
    // stack: x ^ y ^ z
    jump


global F1:
    jumpdest
    // stack: 1, 2, 3
    dup1
    // stack: 1, 1, 2, 3
    swap2
    // stack: 2, 1, 1, 3
    and
    // stack: 0, 1, 3
    swap2
    // stack: 3, 1, 0
    swap1
    // stack: 1, 3, 0
    not
    // stack: -2, 3, 0
    push 0x100000000
    // stack: 4294967296, -2, 3, 0
    swap1
    // stack: -2, 4294967296, 3, 0
    mod
    // stack: 4294967294, 3, 0
    and
    // stack: 2, 0
    or
    // stack: 2
    jump


global F2:
    jumpdest
    // stack: 1, 2, 3
    swap1
    // stack: 2, 1, 3
    not
    // stack: -3, 1, 3
    push 0x100000000
    // stack: 4294967296, -3, 1, 3
    swap1
    // stack: -3, 4294967296, 1, 3
    mod
    // stack: 4294967293, 1, 3
    or
    // stack: 4294967293, 3
    xor
    // stack: 4294967294
    jump


global F3:
    jumpdest
    // stack: 1, 2, 3
    dup3
    // stack: 1, 2, 3, 3
    swap3
    // stack: 3, 2, 3, 1
    not
    // stack: -4, 2, 3, 1
    push 0x100000000
    // stack: 4294967296, -4, 2, 3, 1
    swap1
    // stack: -4, 4294967296, 2, 3, 1
    mod
    // stack: 4294967292, 2, 3, 1
    and
    // stack: 0, 3, 1
    swap2
    // stack: 1, 3, 0
    and
    // stack: 1, 0
    or
    // stack: 1
    jump 


global F4:
    jumpdest 
    // stack: 1, 2, 3
    swap2
    // stack: 3, 2, 1
    not
    // stack: -4, 2, 1
    push 0x100000000
    // stack: 4294967296, -4, 2, 1
    swap1
    // stack: -4, 4294967296, 2, 1
    mod
    // stack: 4294967292, 2, 1
    or
    // stack: 4294967294, 1
    xor
    // stack: 4294967295
    jump