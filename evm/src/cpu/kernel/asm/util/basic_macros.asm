%macro jump(dst)
    PUSH $dst
    jump
%endmacro

%macro jumpi(dst)
    PUSH $dst
    jumpi
%endmacro

%macro pop2
    %rep 2
        POP
    %endrep
%endmacro

%macro pop3
    %rep 3
        POP
    %endrep
%endmacro

%macro pop4
    %rep 4
        POP
    %endrep
%endmacro

%macro pop5
    %rep 5
        POP
    %endrep
%endmacro

%macro pop6
    %rep 6
        POP
    %endrep
%endmacro

%macro pop7
    %rep 7
        POP
    %endrep
%endmacro

%macro pop8
    %rep 8
        POP
    %endrep
%endmacro

%macro neq
    // stack: x, y
    EQ
    // stack: x == y
    ISZERO
    // stack: x != y
%endmacro

%macro and_const(c)
    // stack: input, ...
    PUSH $c
    AND
    // stack: input & c, ...
%endmacro

%macro add_const(c)
    // stack: input, ...
    PUSH $c
    ADD
    // stack: input + c, ...
%endmacro

// Slightly inefficient as we need to swap the inputs.
// Consider avoiding this in performance-critical code.
%macro sub_const(c)
    // stack: input, ...
    PUSH $c
    // stack: c, input, ...
    SWAP1
    // stack: input, c, ...
    SUB
    // stack: input - c, ...
%endmacro

%macro mul_const(c)
    // stack: input, ...
    PUSH $c
    MUL
    // stack: input * c, ...
%endmacro

// Slightly inefficient as we need to swap the inputs.
// Consider avoiding this in performance-critical code.
%macro div_const(c)
    // stack: input, ...
    PUSH $c
    // stack: c, input, ...
    SWAP1
    // stack: input, c, ...
    DIV
    // stack: input / c, ...
%endmacro

// Slightly inefficient as we need to swap the inputs.
// Consider avoiding this in performance-critical code.
%macro mod_const(c)
    // stack: input, ...
    PUSH $c
    // stack: c, input, ...
    SWAP1
    // stack: input, c, ...
    MOD
    // stack: input % c, ...
%endmacro

%macro shl_const(c)
    // stack: input, ...
    PUSH $c
    SHL
    // stack: input << c, ...
%endmacro

%macro shr_const(c)
    // stack: input, ...
    PUSH $c
    SHR
    // stack: input >> c, ...
%endmacro

%macro eq_const(c)
    // stack: input, ...
    PUSH $c
    EQ
    // stack: input == c, ...
%endmacro

%macro lt_const(c)
    // stack: input, ...
    PUSH $c
    // stack: c, input, ...
    GT // Check it backwards: (input < c) == (c > input)
    // stack: input < c, ...
%endmacro

%macro le_const(c)
    // stack: input, ...
    PUSH $c
    // stack: c, input, ...
    LT ISZERO // Check it backwards: (input <= c) == !(c < input)
    // stack: input <= c, ...
%endmacro

%macro gt_const(c)
    // stack: input, ...
    PUSH $c
    // stack: c, input, ...
    LT // Check it backwards: (input > c) == (c < input)
    // stack: input >= c, ...
%endmacro

%macro ge_const(c)
    // stack: input, ...
    PUSH $c
    // stack: c, input, ...
    GT ISZERO // Check it backwards: (input >= c) == !(c > input)
    // stack: input >= c, ...
%endmacro

%macro consume_gas_const(c)
    PUSH $c
    CONSUME_GAS
%endmacro

// If pred is zero, yields z; otherwise, yields nz
%macro select
    // stack: pred, nz, z
    ISZERO
    // stack: pred == 0, nz, z
    DUP1
    // stack: pred == 0, pred == 0, nz, z
    ISZERO
    // stack: pred != 0, pred == 0, nz, z
    SWAP3
    // stack: z, pred == 0, nz, pred != 0
    MUL
    // stack: (pred == 0) * z, nz, pred != 0
    SWAP2
    // stack: pred != 0, nz, (pred == 0) * z
    MUL
    // stack: (pred != 0) * nz, (pred == 0) * z
    ADD
    // stack: (pred != 0) * nz + (pred == 0) * z
%endmacro

// If pred, yields x; otherwise, yields y
// Assumes pred is boolean (either 0 or 1).
%macro select_bool
    // stack: pred, y, x
    DUP1
    // stack: pred, pred, y, x
    ISZERO
    // stack: notpred, pred, y, x
    SWAP3
    // stack: x, pred, y, notpred
    MUL
    // stack: pred * x, y, notpred
    SWAP2
    // stack: notpred, y, pred * x
    MUL
    // stack: notpred * y, pred * x
    ADD
    // stack: notpred * y + pred * x
%endmacro

%macro square
    // stack: x
    DUP1
    // stack: x, x
    MUL
    // stack: x^2
%endmacro

%macro min
    // stack: x, y
    DUP2
    DUP2
    // stack: x, y, x, y
    LT
    // stack: x < y, x, y
    %select_bool
    // stack: min
%endmacro

%macro max
    // stack: x, y
    DUP2
    DUP2
    // stack: x, y, x, y
    GT
    // stack: x > y, x, y
    %select_bool
    // stack: max
%endmacro

%macro as_u32
    %and_const(0xffffffff)
%endmacro

%macro as_u64
    %and_const(0xffffffffffffffff)
%endmacro

%macro not_u32
    // stack: x
    PUSH 0xffffffff
    // stack: 0xffffffff, x
    SUB
    // stack: 0xffffffff - x
%endmacro

// u32 addition (discarding 2^32 bit)
%macro add_u32
    // stack: x, y
    ADD
    // stack: x + y
    %as_u32
    // stack: (x + y) & u32::MAX
%endmacro

%macro add3_u32
    // stack: x , y , z
    ADD
    // stack: x + y , z
    ADD
    // stack: x + y + z
    %as_u32
%endmacro

%macro increment
    %add_const(1)
%endmacro

%macro decrement
    %sub_const(1)
%endmacro

%macro div2
    %div_const(2)
%endmacro

%macro iseven
    %mod_const(2)
    ISZERO
%endmacro

// given u32 bytestring abcd return dcba
%macro reverse_bytes_u32
    // stack: abcd
    DUP1
    PUSH 28
    BYTE
    // stack:                a, abcd
    DUP2
    PUSH 29
    BYTE
    %shl_const(8)
    // stack:            b0, a, abcd 
    DUP3
    PUSH 30
    BYTE
    %shl_const(16)
    // stack:       c00, b0, a, abcd
    SWAP3
    PUSH 31
    BYTE
    %shl_const(24)
    // stack: d000, b0, a, c00
    OR 
    OR
    OR
    // stack: dcba
%endmacro

%macro reverse_bytes_u64
    // stack: word
    DUP1
    // stack: word, word
    %and_const(0xffffffff)
    // stack: word_lo, word
    SWAP1
    // stack: word, word_lo
    %shr_const(32)
    // stack: word_hi, word_lo
    %reverse_bytes_u32
    // stack: word_hi_inverted, word_lo
    SWAP1
    // stack: word_lo, word_hi_inverted
    %reverse_bytes_u32
    // stack: word_lo_inverted, word_hi_inverted
    %shl_const(32)
    OR
    // stack: word_inverted
%endmacro

// Combine four big-endian u64s into a u256.
%macro u64s_to_u256
    // stack: a, b, c, d
    %rep 3
        %shl_const(64)
        OR
    %endrep
    // stack: a || b || c || d
%endmacro
