// 32-bit right rotation
%macro rotr(rot)
    // stack: value
    PUSH $rot
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
    %as_u32
    // stack: (value << (32 - rot)) % (1 << 32), value >> rot
    ADD
%endmacro

%macro sha2_sigma_0
    // stack: x
    DUP1
    // stack: x, x
    %rotr(7)
    // stack: rotr(x, 7), x
    %stack (rotated, x) -> (x, x, rotated)
    // stack: x, x, rotr(x, 7)
    %rotr(18)
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
    %rotr(17)
    // stack: rotr(x, 17), x
    %stack (rotated, x) -> (x, x, rotated)
    // stack: x, x, rotr(x, 17)
    %rotr(19)
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
    %rotr(2)
    // stack: rotr(x, 2), x
    %stack (rotated, x) -> (x, x, rotated)
    // stack: x, x, rotr(x, 2)
    %rotr(13)
    // stack: rotr(x, 13), x, rotr(x, 2)
    SWAP1
    // stack: x, rotr(x, 13), rotr(x, 2)
    %rotr(22)
    // stack: rotr(x, 22), rotr(x, 13), rotr(x, 2)
    XOR
    XOR
%endmacro

%macro sha2_bigsigma_1
    // stack: x
    DUP1
    // stack: x, x
    %rotr(6)
    // stack: rotr(x, 6), x
    %stack (rotated, x) -> (x, x, rotated)
    // stack: x, x, rotr(x, 6)
    %rotr(11)
    // stack: rotr(x, 11), x, rotr(x, 6)
    SWAP1
    // stack: x, rotr(x, 11), rotr(x, 6)
    %rotr(25)
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
