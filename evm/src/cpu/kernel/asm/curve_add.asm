// #define N 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47 // BN254 base field order

global ec_add:
    PUSH 2
    PUSH 1
    PUSH 0
    PUSH 0
    JUMPDEST
    // stack: x0, y0, x1, y1, retdest
    DUP2
    // stack: y0, x0, y0, x1, y1, retdest
    DUP2
    // stack: x0, y0, x0, y0, x1, y1, retdest
    ISZERO
    // stack: x0==0, y0, x0, y0, x1, y1, retdest
    SWAP1
    // stack: y0, x0==0, x0, y0, x1, y1, retdest
    ISZERO
    // stack: y0==0, x0==0, x0, y0, x1, y1, retdest
    AND
    // stack: y0==0 & x0==0, x0, y0, x1, y1, retdest
    PUSH ec_add_first_zero
    // stack: ec_add_first_zero, y0==0 & x0==0, x0, y0, x1, y1, retdest
    JUMPI
    // stack: x0, y0, x1, y1, retdest
    DUP4
    // stack: y1, x0, y0, x1, y1, retdest
    DUP4
    // stack: x1, y1, x0, y0, x1, y1, retdest
    ISZERO
    // stack: x1==0, y1, x0, y0, x1, y1, retdest
    SWAP1
    // stack: y1, x1==0, x0, y0, x1, y1, retdest
    ISZERO
    // stack: y1==0, x1==0, x0, y0, x1, y1, retdest
    AND
    // stack: y1==0 & x1==0, x0, y0, x1, y1, retdest
    PUSH ec_add_snd_zero
    // stack: ec_add_snd_zero, y1==0 & x1==0, x0, y0, x1, y1, retdest
    JUMPI
    // stack: x0, y0, x1, y1, retdest
    DUP4
    // stack: y1, x0, y0, x1, y1, retdest
    DUP4
    // stack: x1, y1, x0, y0, x1, y1, retdest
    DUP4
    // stack: y0, x1, y1, x0, y0, x1, y1, retdest
    DUP4
    // stack: x0, y0, x1, y1, x0, y0, x1, y1, retdest
    %ec_check
    // stack: isValid(x0, y0), x1, y1, x0, y0, x1, y1, retdest
    PUSH ec_add_valid_first_point
    // stack: ec_add_valid_first_point, isValid(x0, y0), x1, y1, x0, y0, x1, y1, retdest
    JUMPI
    // stack: x1, y1, x0, y0, x1, y1, retdest
    POP
    // stack: y1, x0, y0, x1, y1, retdest
    POP
    // stack: x0, y0, x1, y1, retdest
    POP
    // stack: y0, x1, y1, retdest
    POP
    // stack: x1, y1, retdest
    POP
    // stack: y1, retdest
    POP
    // stack: retdest
    JUMP

// Assumption (x0,y0) == (0,0)
ec_add_first_zero:
    JUMPDEST
    // stack: x0, y0, x1, y1, retdest
    POP
    // stack: y0, x1, y1, retdest
    POP
    // stack: x1, y1, retdest
    DUP2
    // stack: y1, x1, y1, retdest
    DUP2
    // stack: x1, y1, x1, y1, retdest
    ISZERO
    // stack: x1==0, y1, x1, y1, retdest
    SWAP1
    // stack: y1, x1==0, x1, y1, retdest
    ISZERO
    // stack: y1==0, x1==0, x1, y1, retdest
    AND
    // stack: y1==0 & x1==0, x1, y1, retdest
    PUSH ret_zero
    // stack: ret_zero, y1==0 & x1==0, x1, y1, retdest
    JUMPI
    // stack: x1, y1, retdest
    DUP2
    // stack: y1, x1, y1, retdest
    DUP2
    // stack: x1, y1, x1, y1, retdest
    %ec_check
    // stack: isValid(x1, y1), x1, y1, retdest
    PUSH ec_noop
    // stack: ec_noop, isValid(x1, y1), x1, y1, retdest
    JUMPI
    // stack: x1, y1, retdest
    POP
    // stack: y1, retdest
    POP
    // stack: retdest
    JUMP

// Assumption (x1,y1) == (0,0) and (x0,y0) != (0,0)
ec_add_snd_zero:
    JUMPDEST
    // stack: x0, y0, x1, y1, retdest
    SWAP2
    // stack: x1, y0, x0, y1, retdest
    POP
    // stack: y0, x0, y1, retdest
    SWAP2
    // stack: y1, x0, y0, retdest
    POP
    // stack: x0, y0, retdest
    DUP2
    // stack: y0, x0, y0, retdest
    DUP2
    // stack: x0, y0, x0, y0, retdest
    %ec_check
    // stack: isValid(x0, y0), x0, y0, retdest
    PUSH ec_noop
    // stack: ec_noop, isValid(x0, y0), x0, y0, retdest
    JUMPI
    // stack: x0, y0, retdest
    POP
    // stack: y0, retdest
    POP
    // stack: retdest
    JUMP

ec_noop:
    JUMPDEST
    // x, y, retdest
    SWAP1
    // y, x, retdest
    SWAP2
    // retdest, x, y
    JUMP

ret_zero:
    JUMPDEST
    // stack: x, y, retdest
    POP
    // stack: y, retdest
    POP
    // stack: retdest
    PUSH 0
    // stack: 0, retdest
    PUSH 0
    // stack: 0, 0, retdest
    SWAP2
    // stack:  0, retdest, 0
    SWAP1
    // stack:  retdest, 0, 0
    JUMP


// Assumption: (x0,y0) is a valid point.
ec_add_valid_first_point:
    JUMPDEST
    // stack: x1, y1, x0, y0, x1, y1, retdest
    %ec_check
    // stack: isValid(x1, y1), x0, y0, x1, y1, retdest
    PUSH ec_add_valid_points
    // stack: ec_add_valid_points, isValid(x1, y1), x0, y0, x1, y1, retdest
    JUMPI
    // stack: x0, y0, x1, y1, retdest
    POP
    // stack: y0, x1, y1, retdest
    POP
    // stack: x1, y1, retdest
    POP
    // stack: y1, retdest
    POP
    // stack: retdest
    JUMP

// Assumption: (x0,y0) and (x1,y1) are valid points.
global ec_add_valid_points:
    JUMPDEST
    // stack: x0, y0, x1, y1, retdest
    DUP3
    // stack: x1, x0, y0, x1, y1, retdest
    DUP2
    // stack: x0, x1, x0, y0, x1, y1, retdest
    EQ
    // stack: x0 == x1, x0, y0, x1, y1, retdest
    PUSH ec_add_equal_first_coord
    // stack: ec_add_equal_first_coord, x0 == x1, x0, y0, x1, y1, retdest
    JUMPI
    // stack: x0, y0, x1, y1, retdest
    DUP4
    // stack: y1, x0, y0, x1, y1, retdest
    DUP3
    // stack: y0, y1, x0, y0, x1, y1, retdest
    %submod
    // stack: y0 - y1, x0, y0, x1, y1, retdest
    DUP4
    // stack: x1, y0 - y1, x0, y0, x1, y1, retdest
    DUP3
    // stack: x0, x1, y0 - y1, x0, y0, x1, y1, retdest
    %submod
    // stack: x0 - x1, y0 - y1, x0, y0, x1, y1, retdest
    %moddiv
    // stack: lambda, x0, y0, x1, y1, retdest
    PUSH ec_add_valid_points_with_lambda
    // stack: ec_add_valid_points_with_lambda, lambda, x0, y0, x1, y1, retdest
    JUMP

ec_add_valid_points_with_lambda:
    JUMPDEST
    // stack: lambda, x0, y0, x1, y1, retdest
    DUP2
    // stack: x0, lambda, x0, y0, x1, y1, retdest
    DUP5
    // stack: x1, x0, lambda, x0, y0, x1, y1, retdest
    PUSH 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
    // stack: N, x1, x0, lambda, x0, y0, x1, y1, retdest
    DUP4
    // stack: lambda, N, x1, x0, lambda, x0, y0, x1, y1, retdest
    DUP1
    // stack: lambda, lambda, N, x1, x0, lambda, x0, y0, x1, y1, retdest
    MULMOD
    // stack: lambda^2, x1, x0, lambda, x0, y0, x1, y1, retdest
    %submod
    // stack: lambda^2 - x1, x0, lambda, x0, y0, x1, y1, retdest
    %submod
    // stack: x2, lambda, x0, y0, x1, y1, retdest
    PUSH 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
    // stack: N, x2, lambda, x0, y0, x1, y1, retdest
    DUP2
    // stack: x2, N, x2, lambda, x0, y0, x1, y1, retdest
    DUP7
    // stack: x1, x2, N, x2, lambda, x0, y0, x1, y1, retdest
    %submod
    // stack: x1 - x2, N, x2, lambda, x0, y0, x1, y1, retdest
    DUP4
    // stack: lambda, x1 - x2, N, x2, lambda, x0, y0, x1, y1, retdest
    MULMOD
    // stack: lambda * (x1 - x2), x2, lambda, x0, y0, x1, y1, retdest
    DUP7
    // stack: y1, lambda * (x1 - x2), x2, lambda, x0, y0, x1, y1, retdest
    SWAP1
    // stack: lambda * (x1 - x2), y1, x2, lambda, x0, y0, x1, y1, retdest
    %submod
    // stack: y2, x2, x0, y0, x1, y1, retdest
    SWAP4
    // stack: x1, x2, x0, y0, y2, y1, retdest
    POP
    // stack: x2, x0, y0, y2, y1, retdest
    SWAP4
    // stack: y1, x0, y0, y2, x2, retdest
    POP
    // stack: x0, y0, y2, x2, retdest
    POP
    // stack: y0, y2, x2, retdest
    POP
    // stack: y2, x2, retdest
    SWAP2
    // stack: retdest, x2, y2
    JUMP

ec_add_equal_first_coord:
    JUMPDEST
    // stack: x0, y0, x1, y1, retdest with x0 == x1
    PUSH 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
    // stack: N, x0, y0, x1, y1, retdest
    DUP3
    // stack: y0, N, x0, y0, x1, y1, retdest
    DUP6
    // stack: y1, y0, N, x0, y0, x1, y1, retdest
    ADDMOD
    // stack: y1 + y0, x0, y0, x1, y1, retdest
    PUSH ec_add_equal_points
    // stack: ec_add_equal_points, y1 + y0, x0, y0, x1, y1, retdest
    JUMPI
    // stack: x0, y0, x1, y1, retdest
    POP
    // stack: y0, x1, y1, retdest
    POP
    // stack: x1, y1, retdest
    POP
    // stack: y1, retdest
    POP
    // stack: retdest
    PUSH 0
    // stack: 0, retdest
    PUSH 0
    // stack: 0, 0, retdest
    SWAP2
    // stack: retdest, 0, 0
    JUMP


// Assumption: x0 == x1 and y0 == y1
ec_add_equal_points:
    JUMPDEST
    // stack: x0, y0, x1, y1, retdest
    PUSH 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
    // stack: N, x0, y0, x1, y1, retdest
    PUSH 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
    // stack: N, N, x0, y0, x1, y1, retdest
    DUP3
    // stack: x0, N, N, x0, y0, x1, y1, retdest
    DUP1
    // stack: x0, x0, N, N, x0, y0, x1, y1, retdest
    MULMOD
    // stack: x0^2, N, x0, y0, x1, y1, retdest with
    PUSH 0x183227397098d014dc2822db40c0ac2ecbc0b548b438e5469e10460b6c3e7ea5 // 3/2 in the base field
    // stack: 3/2, x0^2, N, x0, y0, x1, y1, retdest
    MULMOD
    // stack: 3/2 * x0^2, x0, y0, x1, y1, retdest
    DUP3
    // stack: y0, 3/2 * x0^2, x0, y0, x1, y1, retdest
    %moddiv
    // stack: lambda, x0, y0, x1, y1, retdest
    PUSH ec_add_valid_points_with_lambda
    // stack: ec_add_valid_points_with_lambda, lambda, x0, y0, x1, y1, retdest
    JUMP

// Assumption: (x0,y0) is a valid point.
global ec_double:
    JUMPDEST
    // stack: x0, y0, retdest
    DUP2
    // stack: y0, x0, y0, retdest
    DUP2
    // stack: x0, y0, x0, y0, retdest
    PUSH ec_add_equal_points
    // stack: ec_add_equal_points, x0, y0, x0, y0, retdest
    JUMP

// Assumption: x, y < N and 2N < 2^256.
// Note: Doesn't hold for Secp256k1 base field.
%macro submod
    // stack: x, y
    PUSH 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
    // stack: N, x, y
    ADD
    // stack: N + x, y // Doesn't overflow since 2N < 2^256
    SUB
    // stack: N + x - y // Doesn't underflow since y < N
    PUSH 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
    // stack: N, N + x - y
    SWAP1
    // stack: N + x - y, N
    MOD
    // stack: (N + x - y) % N = (x-y) % N
%endmacro

%macro ec_check
    // stack: x0, y0
    PUSH 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
    // stack: N, x0, y0
    DUP2
    // stack: x0, N, x0, y0
    LT
    // stack: x0 < N, x0, y0
    PUSH 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
    // stack: N, x0 < N, x0, y0
    DUP4
    // stack: y0, N, x0 < N, x0, y0
    LT
    // stack: y0 < N, x0 < N, x0, y0
    AND
    // stack: (y0 < N) & (x0 < N), x0, y0
    SWAP2
    // stack: y0, x0, (y0 < N) & (x0 < N), x0
    SWAP1
    // stack: x0, y0, (y0 < N) & (x0 < N)
    PUSH 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
    // stack: N, x0, y0, b
    PUSH 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
    // stack: N, N, x0, y0, b
    SWAP2
    // stack: x0, N, N, y0, b
    PUSH 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
    // stack: N, x0, N, N, y0, b
    DUP2
    // stack: x0, N, x0, N, N, y0, b
    DUP1
    // stack: x0, x0, N, x0, N, N, y0, b
    MULMOD
    // stack: x0^2 % N, x0, N, N, y0, b
    MULMOD
    // stack: x0^3 % N, N, y0, b
    PUSH 3
    // stack: 3, x0^3 % N, N, y0, b
    ADDMOD
    // stack: (x0^3 + 3) % N, y0, b
    SWAP1
    // stack: y0, (x0^3 + 3) % N, b
    PUSH 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
    // stack: N, y0, (x0^3 + 3) % N, b
    SWAP1
    // stack: y0, N, (x0^3 + 3) % N, b
    DUP1
    // stack: y0, y0, N, (x0^3 + 3) % N, b
    MULMOD
    // stack: y0^2 % N, (x0^3 + 3) % N, b
    EQ
    // stack: y0^2 % N == (x0^3 + 3) % N, b
    AND
    // stack: y0^2 % N == (x0^3 + 3) % N & (x < N) & (y < N)
%endmacro
