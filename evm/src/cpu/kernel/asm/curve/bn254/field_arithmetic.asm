global fp2_add:
    // stack: a0, a1, b0, b1
    PUSH pr
    SWAP2
    SWAP3
    // stack: b0, a0, pr, a1, b1
    ADDMOD
    // stack: c0, a1, b1
    PUSH pr
    SWAP2
    // stack: a1, c0, pr, b1
    SWAP1
    SWAP3 
    // stack: b1, a1, pr, c0
    ADDMOD
    // stack: c1, c0
    SWAP1
    // stack: c0, c1  


// [a0, a1]*[b0, b1] = [a0b0 - a1b1, a0b1 + a1b0]

global fp2_mul:
    // stack:                 a0, a1, b0, b1
    PUSH pr
    PUSH pr
    DUP3
    DUP6
    MULMOD
    // stack:       a0b1, pr, a0, a1, b0, b1
    PUSH pr
    DUP5
    DUP8
    MULMOD
    // stack: a1b0, a0b1, pr, a0, a1, b0, b1
    ADDMOD
    // stack:             c1, a0, a1, b0, b1
    SWAP4
    // stack:             b1, a0, a1, b0, c1
    SWAP3
    // stack:             b0, a0, a1, b1, c1
    PUSH pr
    SWAP2
    // stack:         a0, b0, pr, a1, b1, c1 
    MULMOD
    // stack:               a0b0, a1, b1, c1
    SWAP2
    // stack:               b1, a1, a0b0, c1
    PUSH pr
    SWAP2
    // stack:           a1, b1, pr, a0b0, c1
    MULMOD
    // stack:                 a1b1, a0b0, c1
    PUSH pr
    SUB
    // stack:                -a1b1, a0b0, c1
    PUSH pr
    SWAP2
    ADDMOD
    // stack:                         c0, c1

y_mul:
    // stack: a0, a1, a2, a3, a4, a5
    SWAP2
    SWAP4
    SWAP1
    SWAP3
    SWAP5
    SWAP1
    // stack: a4, a5, a0, a1, a2, a3
    %mul_9_plus_i
    // stack: c4, c5, a0, a1, a2, a3

%macro mul_9_plus_i
    // stack:                b0, b1
    PUSH pr
    PUSH pr
    DUP3
    PUSH 9
    // stack: 9, b0, pr, pr, b0, b1
    MULMOD
    // stack:       9b0, pr, b0, b1
    DUP3
    ADDMOD
    // stack:      b1 + 9b0, b0, b1
    SWAP2
    // stack:           b1,  b0, c1
    PUSH pr
    SUB
    // stack:          -b1,  b0, c1
    PUSH pr
    PUSH pr
    SWAP3
    // stack:   b0, pr, -b1, pr, c1   
    PUSH 9
    // stack:    9, b0, pr, -b1, c1 
    MULMOD
    // stack:      9b0,     -b1, c1
    ADDMOD
%endmacro
