%macro jump(dst)
    push $dst
    jump
%endmacro

%macro jumpi(dst)
    push $dst
    jumpi
%endmacro

%macro pop2
    pop
    pop
%endmacro

%macro pop3
    pop
    %pop2
%endmacro

%macro pop4
    %pop2
    %pop2
%endmacro

// If pred is zero, yields z; otherwise, yields nz
%macro select
    // stack: pred, nz, z
    iszero
    // stack: pred == 0, nz, z
    dup1
    // stack: pred == 0, pred == 0, nz, z
    iszero
    // stack: pred != 0, pred == 0, nz, z
    swap3
    // stack: z, pred == 0, nz, pred != 0
    mul
    // stack: (pred == 0) * z, nz, pred != 0
    swap2
    // stack: pred != 0, nz, (pred == 0) * z
    mul
    // stack: (pred != 0) * nz, (pred == 0) * z
    add
    // stack: (pred != 0) * nz + (pred == 0) * z
%endmacro

%macro square
    // stack: x
    dup1
    // stack: x, x
    mul
    // stack: x^2
%endmacro
