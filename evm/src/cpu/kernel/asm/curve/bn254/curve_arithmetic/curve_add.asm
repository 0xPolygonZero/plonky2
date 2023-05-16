// BN254 elliptic curve addition.
// Uses the standard affine addition formula.
global bn_add:
    // stack: x0, y0, x1, y1, retdest
    // Check if points are valid BN254 points.
    DUP2
    // stack: y0, x0, y0, x1, y1, retdest
    DUP2
    // stack: x0, y0, x0, y0, x1, y1, retdest
    %bn_check
    // stack: isValid(x0, y0), x0, y0, x1, y1, retdest
    DUP5
    // stack: x1, isValid(x0, y0), x0, y0, x1, y1, retdest
    DUP5
    // stack: x1, y1, isValid(x0, y0), x0, y0, x1, y1, retdest
    %bn_check
    // stack: isValid(x1, y1), isValid(x0, y0), x0, y0, x1, y1, retdest
    AND
    // stack: isValid(x1, y1) & isValid(x0, y0), x0, y0, x1, y1, retdest
    %jumpi(bn_add_valid_points)
    // stack: x0, y0, x1, y1, retdest

    // Otherwise return
    %pop4
    // stack: retdest
    %bn_invalid_input

// BN254 elliptic curve addition.
// Assumption: (x0,y0) and (x1,y1) are valid points.
global bn_add_valid_points:
    // stack: x0, y0, x1, y1, retdest

    // Check if the first point is the identity.
    DUP2
    // stack: y0, x0, y0, x1, y1, retdest
    DUP2
    // stack: x0, y0, x0, y0, x1, y1, retdest
    %ec_isidentity
    // stack: (x0,y0)==(0,0), x0, y0, x1, y1, retdest
    %jumpi(bn_add_fst_zero)
    // stack: x0, y0, x1, y1, retdest

    // Check if the second point is the identity.
    DUP4
    // stack: y1, x0, y0, x1, y1, retdest
    DUP4
    // stack: x1, y1, x0, y0, x1, y1, retdest
    %ec_isidentity
    // stack: (x1,y1)==(0,0), x0, y0, x1, y1, retdest
    %jumpi(bn_add_snd_zero)
    // stack: x0, y0, x1, y1, retdest

    // Check if both points have the same x-coordinate.
    DUP3
    // stack: x1, x0, y0, x1, y1, retdest
    DUP2
    // stack: x0, x1, x0, y0, x1, y1, retdest
    EQ
    // stack: x0 == x1, x0, y0, x1, y1, retdest
    %jumpi(bn_add_equal_first_coord)
    // stack: x0, y0, x1, y1, retdest

    // Otherwise, we can use the standard formula.
    // Compute lambda = (y0 - y1)/(x0 - x1)
    DUP4
    // stack: y1, x0, y0, x1, y1, retdest
    DUP3
    // stack: y0, y1, x0, y0, x1, y1, retdest
    SUBFP254
    // stack: y0 - y1, x0, y0, x1, y1, retdest
    DUP4
    // stack: x1, y0 - y1, x0, y0, x1, y1, retdest
    DUP3
    // stack: x0, x1, y0 - y1, x0, y0, x1, y1, retdest
    SUBFP254
    // stack: x0 - x1, y0 - y1, x0, y0, x1, y1, retdest
    %divr_fp254
    // stack: lambda, x0, y0, x1, y1, retdest
    %jump(bn_add_valid_points_with_lambda)

// BN254 elliptic curve addition.
// Assumption: (x0,y0) == (0,0)
bn_add_fst_zero:
    // stack: x0, y0, x1, y1, retdest
    // Just return (x1,y1)
    %stack (x0, y0, x1, y1, retdest) -> (retdest, x1, y1)
    JUMP

// BN254 elliptic curve addition.
// Assumption: (x1,y1) == (0,0)
bn_add_snd_zero:
    // stack: x0, y0, x1, y1, retdest

    // Just return (x0,y0)
    %stack (x0, y0, x1, y1, retdest) -> (retdest, x0, y0)
    JUMP

// BN254 elliptic curve addition.
// Assumption: lambda = (y0 - y1)/(x0 - x1)
bn_add_valid_points_with_lambda:
    // stack: lambda, x0, y0, x1, y1, retdest

    // Compute x2 = lambda^2 - x1 - x0
    DUP2
    // stack: x0, lambda, x0, y0, x1, y1, retdest
    DUP5
    // stack: x1, x0, lambda, x0, y0, x1, y1, retdest
    DUP3
    // stack: lambda, x1, x0, lambda, x0, y0, x1, y1, retdest
    DUP1
    // stack: lambda, lambda, x1, x0, lambda, x0, y0, x1, y1, retdest
    MULFP254
    // stack: lambda^2, x1, x0, lambda, x0, y0, x1, y1, retdest
    SUBFP254
    // stack: lambda^2 - x1, x0, lambda, x0, y0, x1, y1, retdest
    SUBFP254
    // stack: x2, lambda, x0, y0, x1, y1, retdest

    // Compute y2 = lambda*(x1 - x2) - y1
    DUP1
    // stack: x2, x2, lambda, x0, y0, x1, y1, retdest
    DUP6
    // stack: x1, x2, x2, lambda, x0, y0, x1, y1, retdest
    SUBFP254
    // stack: x1 - x2, x2, lambda, x0, y0, x1, y1, retdest
    DUP3
    // stack: lambda, x1 - x2, x2, lambda, x0, y0, x1, y1, retdest
    MULFP254
    // stack: lambda * (x1 - x2), x2, lambda, x0, y0, x1, y1, retdest
    DUP7
    // stack: y1, lambda * (x1 - x2), x2, lambda, x0, y0, x1, y1, retdest
    SWAP1
    // stack: lambda * (x1 - x2), y1, x2, lambda, x0, y0, x1, y1, retdest
    SUBFP254
    // stack: y2, x2, lambda, x0, y0, x1, y1, retdest

    // Return x2,y2
    %stack (y2, x2, lambda, x0, y0, x1, y1, retdest) -> (retdest, x2, y2)
    JUMP

// BN254 elliptic curve addition.
// Assumption: (x0,y0) and (x1,y1) are valid points and x0 == x1
bn_add_equal_first_coord:
    // stack: x0, y0, x1, y1, retdest with x0 == x1

    // Check if the points are equal
    DUP2
    // stack: y0, x0, y0, x1, y1, retdest
    DUP5
    // stack: y1, y0, x0, y0, x1, y1, retdest
    EQ
    // stack: y1 == y0, x0, y0, x1, y1, retdest
    %jumpi(bn_add_equal_points)
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


// BN254 elliptic curve addition.
// Assumption: x0 == x1 and y0 == y1
// Standard doubling formula.
bn_add_equal_points:
    // stack: x0, y0, x1, y1, retdest

    // Compute lambda = 3/2 * x0^2 / y0
    DUP1
    // stack: x0, x0, y0, x1, y1, retdest
    DUP1
    // stack: x0, x0, x0, y0, x1, y1, retdest
    MULFP254
    // stack: x0^2, x0, y0, x1, y1, retdest with
    PUSH 0x183227397098d014dc2822db40c0ac2ecbc0b548b438e5469e10460b6c3e7ea5 // 3/2 in the base field
    // stack: 3/2, x0^2, x0, y0, x1, y1, retdest
    MULFP254
    // stack: 3/2 * x0^2, x0, y0, x1, y1, retdest
    DUP3
    // stack: y0, 3/2 * x0^2, x0, y0, x1, y1, retdest
    %divr_fp254
    // stack: lambda, x0, y0, x1, y1, retdest
    %jump(bn_add_valid_points_with_lambda)

// BN254 elliptic curve doubling.
// Assumption: (x0,y0) is a valid point.
// Standard doubling formula.
global bn_double:
    // stack: x, y, retdest
    DUP2 DUP2 %ec_isidentity
    // stack: (x,y)==(0,0), x, y, retdest
    %jumpi(ec_double_retself)
    DUP2 DUP2
    // stack: x, y, x, y, retdest
    %jump(bn_add_equal_points)

// Check if (x,y) is a valid curve point.
// Returns (range & curve) || ident
//   where
//     range = (x < N) & (y < N) 
//     curve = y^2 == (x^3 + 3) 
//     ident = (x,y) == (0,0)
%macro bn_check
    // stack:                       x, y
    DUP1
    // stack:                    x, x, y
    PUSH @BN_BASE
    // stack:                N , x, x, y
    DUP1
    // stack:             N, N , x, x, y
    DUP5
    // stack:         y , N, N , x, x, y
    LT  
    // stack:         y < N, N , x, x, y
    SWAP2
    // stack:         x , N, y < N, x, y
    LT
    // stack:         x < N, y < N, x, y
    AND
    // stack:                range, x, y
    SWAP2
    // stack:                y, x, range
    DUP2 
    // stack:           x  , y, x, range
    DUP1 
    DUP1
    MULFP254
    MULFP254
    // stack:           x^3, y, x, range
    PUSH 3
    ADDFP254
    // stack:       3 + x^3, y, x, range
    DUP2
    // stack:  y  , 3 + x^3, y, x, range
    DUP1
    MULFP254
    // stack:  y^2, 3 + x^3, y, x, range
    EQ
    // stack:         curve, y, x, range
    SWAP2
    // stack:         x, y, curve, range
    %ec_isidentity
    // stack:       ident , curve, range
    SWAP2
    // stack:       range , curve, ident
    AND
    // stack:       range & curve, ident
    OR
    // stack:                   is_valid
%endmacro

// Return (u256::MAX, u256::MAX) which is used to indicate the input was invalid.
%macro bn_invalid_input
    // stack: retdest
    PUSH @U256_MAX
    // stack: u256::MAX, retdest
    DUP1
    // stack: u256::MAX, u256::MAX, retdest
    SWAP2
    // stack: retdest, u256::MAX, u256::MAX
    JUMP
%endmacro
