// 64-bit right rotation
%macro rotr_64(rot)
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
    PUSH 64
    SUB
    // stack: 64 - rot, value, value >> rot
    SHL
    // stack: value << (64 - rot), value >> rot
    %as_u64
    // stack: (value << (64 - rot)) % (1 << 64), value >> rot
    ADD
%endmacro
