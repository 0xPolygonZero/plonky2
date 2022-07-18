// 32-bit right rotation
%macro rotr
    // stack: rot, value
    dup2
    dup2
    // stack: rot, value, rot, value
    shr
    // stack: value >> rot, rot, value
    swap2
    // stack: value, rot, value >> rot
    swap1
    // stack: rot, value, value >> rot
    push 32
    sub
    // stack: 32 - rot, value, value >> rot
    shl
    // stack: value << (32 - rot), value >> rot
    push 32
    push 1
    swap1
    shl
    // stack: 1 << 32, value << (32 - rot), value >> rot
    swap1
    mod
    // stack: (value << (32 - rot)) % (1 << 32), value >> rot
    add
%endmacro

// 32-bit left rotation
%macro rot,
    // stack: rot, value
    dup2
    dup2
    // stack: rot, value, rot, value
    push 32
    sub
    // stack: 32 - rot, value, rot, value
    shr
    // stack: value >> (32 - rot), rot, value
    swap2
    // stack: value, rot, value >> (32 - rot)
    swap1
    // stack: rot, value, value >> (32 - rot)
    shl
    // stack: value << rot, value >> (32 - rot)
    push 32
    push 1
    swap1
    shl
    // stack: 1 << 32, value << rot, value >> (32 - rot)
    swap1
    mod
    // stack: (value << rot) % (1 << 32), value >> (32 - rot)
    add
%endmacro

global sha2_sigma_0:
    JUMPDEST
    // stack: x
    dup1
    // stack: x, x
    push 7
    %rotr
    // stack: rotr(x, 7), x
    swap1
    // stack: x, rotr(x, 7)
    dup1
    // stack: x, x, rotr(x, 7)
    push 18
    %rotr
    // stack: rotr(x, 18), x, rotr(x, 7)
    swap1
    // stack: x, rotr(x, 18), rotr(x, 7)
    push 3
    shr
    // stack: shr(x, 3), rotr(x, 18), rotr(x, 7)
    xor
    xor

global sha2_sigma_1:
    JUMPDEST
    // stack: x
    dup1
    // stack: x, x
    push 17
    %rotr
    // stack: rotr(x, 17), x
    swap1
    // stack: x, rotr(x, 17)
    dup1
    // stack: x, x, rotr(x, 17)
    push 19
    %rotr
    // stack: rotr(x, 19), x, rotr(x, 17)
    swap1
    // stack: x, rotr(x, 19), rotr(x, 17)
    push 10
    shr
    // stack: shr(x, 10), rotr(x, 19), rotr(x, 17)
    xor
    xor

global sha2_bigsigma_0:
    JUMPDEST
    // stack: x
    dup1
    // stack: x, x
    push 2
    %rotr
    // stack: rotr(x, 2), x
    swap1
    // stack: x, rotr(x, 2)
    dup1
    // stack: x, x, rotr(x, 2)
    push 13
    %rotr
    // stack: rotr(x, 13), x, rotr(x, 2)
    swap1
    // stack: x, rotr(x, 13), rotr(x, 2)
    push 22
    %rotr
    // stack: rotr(x, 22), rotr(x, 13), rotr(x, 2)
    xor
    xor

global sha2_bigsigma_1:
    JUMPDEST
    // stack: x
    dup1
    // stack: x, x
    push 6
    %rotr
    // stack: rotr(x, 6), x
    swap1
    // stack: x, rotr(x, 6)
    dup1
    // stack: x, x, rotr(x, 6)
    push 11
    %rotr
    // stack: rotr(x, 11), x, rotr(x, 6)
    swap1
    // stack: x, rotr(x, 11), rotr(x, 6)
    push 25
    %rotr
    // stack: rotr(x, 25), rotr(x, 11), rotr(x, 6)
    xor
    xor

global sha2_choice:
    JUMPDEST
    // stack: x, y, z
    dup1
    // stack: x, x, y, z
    swap2
    // stack: y, x, x, z
    and
    // stack: x and y, x, z
    swap2
    // stack: z, x, x and y
    swap1
    // stack: x, z, x and y
    not
    // stack: not x, z, x and y
    and
    // stack: (not x) and z, x and y
    or

global sha2_majority:
    JUMPDEST
    // stack: x, y, z
    dup3
    dup3
    dup3
    // stack: x, y, z, x, y, z
    and
    // stack: x and y, z, x, y, z
    swap2
    // stack: x, x and y, z, y, z
    swap1
    // stack: x and y, x, z, y, z
    swap2
    // stack: z, x, x and y, y, z
    and
    // stack: x and z, x and y, y, z
    swap2
    // stack: y, x and z, x and y, z
    swap1
    // stack: x and z, y, x and y, z
    swap3
    // stack: z, y, x and z, x and y
    and
    // stack: y and z, x and z, x and y
    or
    or

    