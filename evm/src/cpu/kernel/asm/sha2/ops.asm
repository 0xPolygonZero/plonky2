// u32 addition (discarding 2^32 bit)
%macro add_u32
    // stack: x, y
    add
    // stack: x + y
    dup1
    // stack: x + y, x + y
    %shr_const(32)
    // stack: (x + y) >> 32, x + y
    %shl_const(32)
    // stack: ((x + y) >> 32) << 32, x + y
    swap1
    // stack: x + y, ((x + y) >> 32) << 32
    sub
    // stack: x + y - ((x + y) >> 32) << 32
%endmacro


// 32-bit right rotation
%macro rotr
    // stack: rot, value
    dup2
    dup2
    // stack: rot, value, rot, value
    shr
    // stack: value >> rot, rot, value
    %stack (shifted, rot, value) -> (rot, value, shifted)
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
%macro rotl
    // stack: rot, value
    dup2
    dup2
    // stack: rot, value, rot, value
    push 32
    sub
    // stack: 32 - rot, value, rot, value
    shr
    // stack: value >> (32 - rot), rot, value
    %stack (shifted, rot, value) -> (rot, value, shifted)
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

%macro sha2_sigma_0
    // stack: x
    dup1
    // stack: x, x
    push 7
    %rotr
    // stack: rotr(x, 7), x
    %stack (rotated, x) -> (x, x, rotated)
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
%endmacro

%macro sha2_sigma_1
    // stack: x
    dup1
    // stack: x, x
    push 17
    %rotr
    // stack: rotr(x, 17), x
    %stack (rotated, x) -> (x, x, rotated)
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
%endmacro

%macro sha2_bigsigma_0
    // stack: x
    dup1
    // stack: x, x
    push 2
    %rotr
    // stack: rotr(x, 2), x
    %stack (rotated, x) -> (x, x, rotated)
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
%endmacro

%macro sha2_bigsigma_1
    // stack: x
    dup1
    // stack: x, x
    push 6
    %rotr
    // stack: rotr(x, 6), x
    %stack (rotated, x) -> (x, x, rotated)
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
%endmacro

%macro sha2_choice
    // stack: x, y, z
    dup1
    // stack: x, x, y, z
    not
    // stack: not x, x, y, z
    %stack (notx, x, y, z) -> (notx, z, x, y)
    // stack: not x, z, x, y
    and
    // stack: (not x) and z, x, y
    %stack (nxz, x, y) -> (x, y, nxz)
    // stack: x, y, (not x) and z
    and
    // stack: x and y, (not x) and z
    or
%endmacro

%macro sha2_majority
    // stack: x, y, z
    dup3
    dup3
    dup3
    // stack: x, y, z, x, y, z
    and
    // stack: x and y, z, x, y, z
    swap2
    // stack: x, z, x and y, y, z
    and
    // stack: x and z, x and y, y, z
    swap2
    // stack: y, x and y, x and z, z
    swap1
    // stack: x and y, y, x and z, z
    swap3
    // stack: z, y, x and z, x and y
    and
    // stack: y and z, x and z, x and y
    or
    or
%endmacro
    