// #define N 0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141 // Secp256k1 scalar field order

// Secp256k1 elliptic curve addition.
// Assumption: (x0,y0) and (x1,y1) are valid points.
global ec_add_valid_points_secp:
    // stack: x0, y0, x1, y1, retdest

    // Check if the first point is the identity.
    DUP2
    // stack: y0, x0, y0, x1, y1, retdest
    DUP2
    // stack: x0, y0, x0, y0, x1, y1, retdest
    %ec_isidentity
    // stack: (x0,y0)==(0,0), x0, y0, x1, y1, retdest
    %jumpi(ec_add_first_zero)
    // stack: x0, y0, x1, y1, retdest

    // Check if the first point is the identity.
    DUP4
    // stack: y1, x0, y0, x1, y1, retdest
    DUP4
    // stack: x1, y1, x0, y0, x1, y1, retdest
    %ec_isidentity
    // stack: (x1,y1)==(0,0), x0, y0, x1, y1, retdest
    %jumpi(ec_add_snd_zero)
    // stack: x0, y0, x1, y1, retdest

    // Check if both points have the same x-coordinate.
    DUP3
    // stack: x1, x0, y0, x1, y1, retdest
    DUP2
    // stack: x0, x1, x0, y0, x1, y1, retdest
    EQ
    // stack: x0 == x1, x0, y0, x1, y1, retdest
    %jumpi(ec_add_equal_first_coord)
    // stack: x0, y0, x1, y1, retdest

    // Otherwise, we can use the standard formula.
    // Compute lambda = (y0 - y1)/(x0 - x1)
    DUP4
    // stack: y1, x0, y0, x1, y1, retdest
    DUP3
    // stack: y0, y1, x0, y0, x1, y1, retdest
    %submod_secp_base
    // stack: y0 - y1, x0, y0, x1, y1, retdest
    DUP4
    // stack: x1, y0 - y1, x0, y0, x1, y1, retdest
    DUP3
    // stack: x0, x1, y0 - y1, x0, y0, x1, y1, retdest
    %submod_secp_base
    // stack: x0 - x1, y0 - y1, x0, y0, x1, y1, retdest
    %moddiv_secp_base
    // stack: lambda, x0, y0, x1, y1, retdest
    %jump(ec_add_valid_points_with_lambda)

// Secp256k1 elliptic curve addition.
// Assumption: (x0,y0) == (0,0)
ec_add_first_zero:
    // stack: x0, y0, x1, y1, retdest

    // Just return (x1,y1)
    %pop2
    // stack: x1, y1, retdest
    SWAP1
    // stack: y1, x1, retdest
    SWAP2
    // stack: retdest, x1, y1
    JUMP

// Secp256k1 elliptic curve addition.
// Assumption: (x1,y1) == (0,0)
ec_add_snd_zero:
    // stack: x0, y0, x1, y1, retdest

    // Just return (x1,y1)
    SWAP2
    // stack: x1, y0, x0, y1, retdest
    POP
    // stack: y0, x0, y1, retdest
    SWAP2
    // stack: y1, x0, y0, retdest
    POP
    // stack: x0, y0, retdest
    SWAP1
    // stack: y0, x0, retdest
    SWAP2
    // stack: retdest, x0, y0
    JUMP

// Secp256k1 elliptic curve addition.
// Assumption: lambda = (y0 - y1)/(x0 - x1)
ec_add_valid_points_with_lambda:
    // stack: lambda, x0, y0, x1, y1, retdest

    // Compute x2 = lambda^2 - x1 - x0
    DUP2
    // stack: x0, lambda, x0, y0, x1, y1, retdest
    DUP5
    // stack: x1, x0, lambda, x0, y0, x1, y1, retdest
    %secp_base
    // stack: N, x1, x0, lambda, x0, y0, x1, y1, retdest
    DUP4
    // stack: lambda, N, x1, x0, lambda, x0, y0, x1, y1, retdest
    DUP1
    // stack: lambda, lambda, N, x1, x0, lambda, x0, y0, x1, y1, retdest
    MULMOD
    // stack: lambda^2, x1, x0, lambda, x0, y0, x1, y1, retdest
    %submod_secp_base
    // stack: lambda^2 - x1, x0, lambda, x0, y0, x1, y1, retdest
    %submod_secp_base
    // stack: x2, lambda, x0, y0, x1, y1, retdest

    // Compute y2 = lambda*(x1 - x2) - y1
    %secp_base
    // stack: N, x2, lambda, x0, y0, x1, y1, retdest
    DUP2
    // stack: x2, N, x2, lambda, x0, y0, x1, y1, retdest
    DUP7
    // stack: x1, x2, N, x2, lambda, x0, y0, x1, y1, retdest
    %submod_secp_base
    // stack: x1 - x2, N, x2, lambda, x0, y0, x1, y1, retdest
    DUP4
    // stack: lambda, x1 - x2, N, x2, lambda, x0, y0, x1, y1, retdest
    MULMOD
    // stack: lambda * (x1 - x2), x2, lambda, x0, y0, x1, y1, retdest
    DUP7
    // stack: y1, lambda * (x1 - x2), x2, lambda, x0, y0, x1, y1, retdest
    SWAP1
    // stack: lambda * (x1 - x2), y1, x2, lambda, x0, y0, x1, y1, retdest
    %submod_secp_base
    // stack: y2, x2, lambda, x0, y0, x1, y1, retdest

    // Return x2,y2
    SWAP5
    // stack: x1, x2, lambda, x0, y0, y2, y1, retdest
    POP
    // stack: x2, lambda, x0, y0, y2, y1, retdest
    SWAP5
    // stack: y1, lambda, x0, y0, y2, x2, retdest
    %pop4
    // stack: y2, x2, retdest
    SWAP2
    // stack: retdest, x2, y2
    JUMP

// Secp256k1 elliptic curve addition.
// Assumption: (x0,y0) and (x1,y1) are valid points and x0 == x1
ec_add_equal_first_coord:
    // stack: x0, y0, x1, y1, retdest with x0 == x1

    // Check if the points are equal
    DUP2
    // stack: y0, x0, y0, x1, y1, retdest
    DUP5
    // stack: y1, y0, x0, y0, x1, y1, retdest
    EQ
    // stack: y1 == y0, x0, y0, x1, y1, retdest
    %jumpi(ec_add_equal_points)
    // stack: x0, y0, x1, y1, retdest

    // Otherwise, one is the negation of the other so we can return (0,0).
    %pop4
    // stack: retdest
    PUSH 0
    // stack: 0, retdest
    PUSH 0
    // stack: 0, 0, retdest
    SWAP2
    // stack: retdest, 0, 0
    JUMP


// Secp256k1 elliptic curve addition.
// Assumption: x0 == x1 and y0 == y1
// Standard doubling formula.
ec_add_equal_points:
    // stack: x0, y0, x1, y1, retdest

    // Compute lambda = 3/2 * x0^2 / y0
    %secp_base
    // stack: N, x0, y0, x1, y1, retdest
    %secp_base
    // stack: N, N, x0, y0, x1, y1, retdest
    DUP3
    // stack: x0, N, N, x0, y0, x1, y1, retdest
    DUP1
    // stack: x0, x0, N, N, x0, y0, x1, y1, retdest
    MULMOD
    // stack: x0^2, N, x0, y0, x1, y1, retdest with
    PUSH 0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffff7ffffe19 // 3/2 in the base field
    // stack: 3/2, x0^2, N, x0, y0, x1, y1, retdest
    MULMOD
    // stack: 3/2 * x0^2, x0, y0, x1, y1, retdest
    DUP3
    // stack: y0, 3/2 * x0^2, x0, y0, x1, y1, retdest
    %moddiv_secp_base
    // stack: lambda, x0, y0, x1, y1, retdest
    %jump(ec_add_valid_points_with_lambda)

// Secp256k1 elliptic curve doubling.
// Assumption: (x0,y0) is a valid point.
// Standard doubling formula.
global ec_double_secp:
    // stack: x0, y0, retdest
    DUP2
    // stack: y0, x0, y0, retdest
    DUP2
    // stack: x0, y0, x0, y0, retdest
    %jump(ec_add_equal_points)

// Push the order of the Secp256k1 scalar field.
%macro secp_base
    PUSH 0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f
%endmacro

// Modular subtraction. Subtraction x-y underflows iff x<x-y, so can be computed as N*(x<x-y) + x-y.
%macro submod_secp_base
    // stack: x, y
    SWAP1
    // stack: y, x
    DUP2
    // stack: x, y, x
    SUB
    // stack: x - y, x
    DUP1
    // stack: x - y, x - y, x
    SWAP2
    // stack: x, x - y, x - y
    LT
    // stack: x < x - y, x - y
    %secp_base
    // stack: N, x < x - y, x - y
    MUL
    // stack: N * (x < x - y), x - y
    ADD
    // (x-y) % N
%endmacro

// Check if (x,y) is a valid curve point.
// Puts y^2 % N == (x^3 + 3) % N & (x < N) & (y < N) || (x,y)==(0,0) on top of the stack.
%macro ec_check_secp
    // stack: x, y
    %secp_base
    // stack: N, x, y
    DUP2
    // stack: x, N, x, y
    LT
    // stack: x < N, x, y
    %secp_base
    // stack: N, x < N, x, y
    DUP4
    // stack: y, N, x < N, x, y
    LT
    // stack: y < N, x < N, x, y
    AND
    // stack: (y < N) & (x < N), x, y
    SWAP2
    // stack: y, x, (y < N) & (x < N), x
    SWAP1
    // stack: x, y, (y < N) & (x < N)
    %secp_base
    // stack: N, x, y, b
    %secp_base
    // stack: N, N, x, y, b
    DUP3
    // stack: x, N, N, x, y, b
    %secp_base
    // stack: N, x, N, N, x, y, b
    DUP2
    // stack: x, N, x, N, N, x, y, b
    DUP1
    // stack: x, x, N, x, N, N, x, y, b
    MULMOD
    // stack: x^2 % N, x, N, N, x, y, b
    MULMOD
    // stack: x^3 % N, N, x, y, b
    PUSH 7
    // stack: 7, x^3 % N, N, x, y, b
    ADDMOD
    // stack: (x^3 + 7) % N, x, y, b
    DUP3
    // stack: y, (x^3 + 7) % N, x, y, b
    %secp_base
    // stack: N, y, (x^3 + 7) % N, x, y, b
    SWAP1
    // stack: y, N, (x^3 + 7) % N, x, y, b
    DUP1
    // stack: y, y, N, (x^3 + 7) % N, x, y, b
    MULMOD
    // stack: y^2 % N, (x^3 + 7) % N, x, y, b
    EQ
    // stack: y^2 % N == (x^3 + 7) % N, x, y, b
    SWAP2
    // stack: y, x, y^2 % N == (x^3 + 7) % N, b
    %ec_isidentity
    // stack: (x,y)==(0,0), y^2 % N == (x^3 + 7) % N, b
    SWAP2
    // stack: b, y^2 % N == (x^3 + 7) % N, (x,y)==(0,0)
    AND
    // stack: y^2 % N == (x^3 + 7) % N & (x < N) & (y < N), (x,y)==(0,0)
    OR
    // stack: y^2 % N == (x^3 + 7) % N & (x < N) & (y < N) || (x,y)==(0,0)
%endmacro