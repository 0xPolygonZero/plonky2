// #define N 0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141 // Secp256k1 scalar field order

// Secp256k1 elliptic curve addition.
// Assumption: (x0,y0) and (x1,y1) are valid points.
global secp_add_valid_points:
    // stack: x0, y0, x1, y1, retdest

    // Check if the first point is the identity.
    DUP2
    // stack: y0, x0, y0, x1, y1, retdest
    DUP2
    // stack: x0, y0, x0, y0, x1, y1, retdest
    %ec_isidentity
    // stack: (x0,y0)==(0,0), x0, y0, x1, y1, retdest
    %jumpi(secp_add_first_zero)
    // stack: x0, y0, x1, y1, retdest

    // Check if the second point is the identity.
    DUP4
    // stack: y1, x0, y0, x1, y1, retdest
    DUP4
    // stack: x1, y1, x0, y0, x1, y1, retdest
    %ec_isidentity
    // stack: (x1,y1)==(0,0), x0, y0, x1, y1, retdest
    %jumpi(secp_add_snd_zero)
    // stack: x0, y0, x1, y1, retdest

    // Check if both points have the same x-coordinate.
    DUP3
    // stack: x1, x0, y0, x1, y1, retdest
    DUP2
    // stack: x0, x1, x0, y0, x1, y1, retdest
    EQ
    // stack: x0 == x1, x0, y0, x1, y1, retdest
    %jumpi(secp_add_equal_first_coord)
// Standard affine addition formula.
global secp_add_valid_points_no_edge_case:
    // stack: x0, y0, x1, y1, retdest
    // Compute lambda = (y0 - y1)/(x0 - x1)
    %secp_base
    // stack: N, x0, y0, x1, y1, retdest
    DUP5
    DUP4
    // stack: y0, y1, N, x0, y0, x1, y1, retdest
    SUBMOD
    // stack: y0 - y1, x0, y0, x1, y1, retdest
    %secp_base
    // stack: N, y0 - y1, x0, y0, x1, y1, retdest
    DUP5
    DUP4
    // stack: x0, x1, N, y0 - y1, x0, y0, x1, y1, retdest
    SUBMOD
    // stack: x0 - x1, y0 - y1, x0, y0, x1, y1, retdest
    %moddiv_secp_base
    // stack: lambda, x0, y0, x1, y1, retdest
    %jump(secp_add_valid_points_with_lambda)

// Secp256k1 elliptic curve addition.
// Assumption: (x0,y0) == (0,0)
secp_add_first_zero:
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
secp_add_snd_zero:
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
secp_add_valid_points_with_lambda:
    // stack: lambda, x0, y0, x1, y1, retdest

    // Compute x2 = lambda^2 - x1 - x0
    %secp_base
    // stack: N, lambda, x0, y0, x1, y1, retdest
    DUP3
    // stack: x0, N, lambda, x0, y0, x1, y1, retdest
    %secp_base
    // stack: N, x0, N, lambda, x0, y0, x1, y1, retdest
    DUP7
    // stack: x1, N, x0, N, lambda, x0, y0, x1, y1, retdest
    %secp_base
    // stack: N, x1, N, x0, N, lambda, x0, y0, x1, y1, retdest
    DUP6
    // stack: lambda, N, x1, N, x0, N, lambda, x0, y0, x1, y1, retdest
    DUP1
    // stack: lambda, lambda, N, x1, N, x0, N, lambda, x0, y0, x1, y1, retdest
    MULMOD
    // stack: lambda^2, x1, N, x0, N, lambda, x0, y0, x1, y1, retdest
    SUBMOD
    // stack: lambda^2 - x1, x0, N, lambda, x0, y0, x1, y1, retdest
    SUBMOD
    // stack: x2, lambda, x0, y0, x1, y1, retdest

    // Compute y2 = lambda*(x1 - x2) - y1
    %secp_base %secp_base %secp_base // Pre-load moduli for incoming SUBMODs
    // stack: N, N, N, x2, lambda, x0, y0, x1, y1, retdest
    DUP4
    // stack: x2, N, N, N, x2, lambda, x0, y0, x1, y1, retdest
    DUP9
    // stack: x1, x2, N, N, N, x2, lambda, x0, y0, x1, y1, retdest
    SUBMOD
    // stack: x1 - x2, N, N, x2, lambda, x0, y0, x1, y1, retdest
    DUP5
    // stack: lambda, x1 - x2, N, N, x2, lambda, x0, y0, x1, y1, retdest
    MULMOD
    // stack: lambda * (x1 - x2), N, x2, lambda, x0, y0, x1, y1, retdest
    DUP8
    // stack: y1, lambda * (x1 - x2), N, x2, lambda, x0, y0, x1, y1, retdest
    SWAP1
    // stack: lambda * (x1 - x2), y1, N, x2, lambda, x0, y0, x1, y1, retdest
    SUBMOD
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
secp_add_equal_first_coord:
    // stack: x0, y0, x1, y1, retdest with x0 == x1

    // Check if the points are equal
    DUP2
    // stack: y0, x0, y0, x1, y1, retdest
    DUP5
    // stack: y1, y0, x0, y0, x1, y1, retdest
    EQ
    // stack: y1 == y0, x0, y0, x1, y1, retdest
    %jumpi(secp_add_equal_points)
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
secp_add_equal_points:
    // Compute lambda = 3/2 * x0^2 / y0
    %stack (x0, y0, x1, y1, retdest) -> (x0, x0, @SECP_BASE, @SECP_BASE, x0, y0, x1, y1, retdest)
    MULMOD
    PUSH 0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffff7ffffe19 // 3/2 in the base field
    MULMOD
    DUP3
    %moddiv_secp_base
    %jump(secp_add_valid_points_with_lambda)

// Secp256k1 elliptic curve doubling.
// Assumption: (x,y) is a valid point.
// Standard doubling formula.
global secp_double:
    // stack: x, y, retdest
    DUP2 DUP2 %ec_isidentity
    // stack: (x,y)==(0,0), x, y, retdest
    %jumpi(ec_double_retself)

    // Compute lambda = 3/2 * x0^2 / y0
    %stack (x, y, retdest) -> (x, x, @SECP_BASE, @SECP_BASE, x, y, x, y, retdest)
    MULMOD
    PUSH 0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffff7ffffe19 // 3/2 in the base field
    MULMOD
    DUP3
    %moddiv_secp_base
    // stack: lambda, x, y, x, y, retdest
    %jump(secp_add_valid_points_with_lambda)

// Push the order of the Secp256k1 scalar field.
%macro secp_base
    PUSH @SECP_BASE
%endmacro

// Modular subtraction.
%macro submod_secp_base
    // stack: x, y
    %stack (x, y) -> (x, y, @SECP_BASE)
    SUBMOD
%endmacro

// Check if (x,y) is a valid curve point.
// Puts y^2 % N == (x^3 + 3) % N & (x < N) & (y < N) || (x,y)==(0,0) on top of the stack.
%macro secp_check
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