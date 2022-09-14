// u32 addition (discarding 2^32 bit)
%macro add_u32
    // stack: x, y
    ADD
    // stack: x + y
    DUP1
    // stack: x + y, x + y
    %shr_const(32)
    // stack: (x + y) >> 32, x + y
    %shl_const(32)
    // stack: ((x + y) >> 32) << 32, x + y
    SWAP1
    // stack: x + y, ((x + y) >> 32) << 32
    SUB
    // stack: x + y - ((x + y) >> 32) << 32
%endmacro


// 32-bit right rotation
%macro rotr
    // stack: rot, value
    DUP2
    DUP2
    // stack: rot, value, rot, value
    SHR
    // stack: value >> rot, rot, value
    %stack (shifted, rot, value) -> (rot, value, shifted)
    // stack: rot, value, value >> rot
    PUSH 32
    SUB
    // stack: 32 - rot, value, value >> rot
    SHL
    // stack: value << (32 - rot), value >> rot
    PUSH 32
    PUSH 1
    SWAP1
    SHL
    // stack: 1 << 32, value << (32 - rot), value >> rot
    SWAP1
    MOD
    // stack: (value << (32 - rot)) % (1 << 32), value >> rot
    ADD
%endmacro

// 32-bit left rotation
%macro rotl
    // stack: rot, value
    DUP2
    DUP2
    // stack: rot, value, rot, value
    PUSH 32
    SUB
    // stack: 32 - rot, value, rot, value
    SHR
    // stack: value >> (32 - rot), rot, value
    %stack (shifted, rot, value) -> (rot, value, shifted)
    // stack: rot, value, value >> (32 - rot)
    SHL
    // stack: value << rot, value >> (32 - rot)
    PUSH 32
    PUSH 1
    SWAP1
    SHL
    // stack: 1 << 32, value << rot, value >> (32 - rot)
    SWAP1
    MOD
    // stack: (value << rot) % (1 << 32), value >> (32 - rot)
    ADD
%endmacro

%macro sha2_sigma_0
    // stack: x
    DUP1
    // stack: x, x
    PUSH 7
    %rotr
    // stack: rotr(x, 7), x
    %stack (rotated, x) -> (x, x, rotated)
    // stack: x, x, rotr(x, 7)
    PUSH 18
    %rotr
    // stack: rotr(x, 18), x, rotr(x, 7)
    SWAP1
    // stack: x, rotr(x, 18), rotr(x, 7)
    PUSH 3
    SHR
    // stack: shr(x, 3), rotr(x, 18), rotr(x, 7)
    XOR
    XOR
%endmacro

%macro sha2_sigma_1
    // stack: x
    DUP1
    // stack: x, x
    PUSH 17
    %rotr
    // stack: rotr(x, 17), x
    %stack (rotated, x) -> (x, x, rotated)
    // stack: x, x, rotr(x, 17)
    PUSH 19
    %rotr
    // stack: rotr(x, 19), x, rotr(x, 17)
    SWAP1
    // stack: x, rotr(x, 19), rotr(x, 17)
    PUSH 10
    SHR
    // stack: shr(x, 10), rotr(x, 19), rotr(x, 17)
    XOR
    XOR
%endmacro

%macro sha2_bigsigma_0
    // stack: x
    DUP1
    // stack: x, x
    PUSH 2
    %rotr
    // stack: rotr(x, 2), x
    %stack (rotated, x) -> (x, x, rotated)
    // stack: x, x, rotr(x, 2)
    PUSH 13
    %rotr
    // stack: rotr(x, 13), x, rotr(x, 2)
    SWAP1
    // stack: x, rotr(x, 13), rotr(x, 2)
    PUSH 22
    %rotr
    // stack: rotr(x, 22), rotr(x, 13), rotr(x, 2)
    XOR
    XOR
%endmacro

%macro sha2_bigsigma_1
    // stack: x
    DUP1
    // stack: x, x
    PUSH 6
    %rotr
    // stack: rotr(x, 6), x
    %stack (rotated, x) -> (x, x, rotated)
    // stack: x, x, rotr(x, 6)
    PUSH 11
    %rotr
    // stack: rotr(x, 11), x, rotr(x, 6)
    SWAP1
    // stack: x, rotr(x, 11), rotr(x, 6)
    PUSH 25
    %rotr
    // stack: rotr(x, 25), rotr(x, 11), rotr(x, 6)
    XOR
    XOR
%endmacro

%macro sha2_choice
    // stack: x, y, z
    DUP1
    // stack: x, x, y, z
    NOT
    // stack: not x, x, y, z
    %stack (notx, x, y, z) -> (notx, z, x, y)
    // stack: not x, z, x, y
    AND
    // stack: (not x) and z, x, y
    %stack (nxz, x, y) -> (x, y, nxz)
    // stack: x, y, (not x) and z
    AND
    // stack: x and y, (not x) and z
    OR
%endmacro

%macro sha2_majority
    // stack: x, y, z
    %stack (xyz: 3) -> (xyz, xyz)
    // stack: x, y, z, x, y, z
    AND
    // stack: x and y, z, x, y, z
    SWAP2
    // stack: x, z, x and y, y, z
    AND
    // stack: x and z, x and y, y, z
    %stack (a: 2, b: 2) -> (b, a)
    // stack: y, z, x and z, x and y
    AND
    // stack: y and z, x and z, x and y
    OR
    OR
%endmacro
    