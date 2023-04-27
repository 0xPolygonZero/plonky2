// Check if (X,Y) is a valid curve point.
// Returns (range & curve) || ident
//   where
//     range = (x < N) & (x_ < N) & (y < N) & (y_ < N)
//     curve = Y^2 == X^3 + 3/(9+i)
//     ident = (X,Y) == (0,0)

%macro bn_check_twisted
    // stack:                x, x_, y, y_
    %bn_check_twisted_range
    // stack:         range, x, x_, y, y_
    %bn_check_twisted_curve
    // stack: curve , range, x, x_, y, y_
    MUL // Cheaper than AND
    // stack: curve & range, x, x_, y, y_
    SWAP4
    // stack: y_, x, x_, y, curve & range
    %bn_check_twisted_ident
    // stack:     ident ,   curve & range
    OR
    // stack:     ident || (curve & range)
%endmacro

%macro bn_check_twisted_range
    // stack:                               x, x_, y, y_
    PUSH @BN_BASE
    // stack:                            N, x, x_, y, y_
    %stack (N) -> (N, N, N, N)
    // stack:                   N, N, N, N, x, x_, y, y_
    DUP8
    // stack:              y_ , N, N, N, N, x, x_, y, y_
    LT  
    // stack:              y_ < N, N, N, N, x, x_, y, y_
    SWAP3
    // stack:              N, N, N, y_ < N, x, x_, y, y_
    DUP7
    // stack:          y , N, N, N, y_ < N, x, x_, y, y_
    LT
    // stack:          y < N, N, N, y_ < N, x, x_, y, y_
    SWAP2
    // stack:          N, N, y < N, y_ < N, x, x_, y, y_
    DUP6
    // stack:     x_ , N, N, y < N, y_ < N, x, x_, y, y_
    LT
    // stack:     x_ < N, N, y < N, y_ < N, x, x_, y, y_
    SWAP1
    // stack:     N, x_ < N, y < N, y_ < N, x, x_, y, y_
    DUP5 
    // stack: x , N, x_ < N, y < N, y_ < N, x, x_, y, y_
    LT
    // stack: x < N, x_ < N, y < N, y_ < N, x, x_, y, y_
    MUL // Cheaper than AND 
    MUL // Cheaper than AND
    MUL // Cheaper than AND
    // stack:                        range, x, x_, y, y_
%endmacro

%macro bn_check_twisted_curve
    // stack:                  range, X, Y
    %stack (range, X: 2, Y: 2) -> (Y, Y, range, X, Y)
    // stack:            Y, Y, range, X, Y
    %mul_fp254_2
    // stack:             Y^2, range, X, Y
    %stack () -> (@BN_TWISTED_RE, @BN_TWISTED_IM)
    // stack:          A, Y^2, range, X, Y
    %stack (A: 2, Y2: 2, range, X: 2) -> (X, X, X, A, Y2, range, X)
    // stack: X, X, X, A, Y^2, range, X, Y
    %mul_fp254_2
    %mul_fp254_2
    // stack:    X^3 , A, Y^2, range, X, Y
    %add_fp254_2
    // stack:    X^3 + A, Y^2, range, X, Y
    %eq_fp254_2
    // stack:           curve, range, X, Y
%endmacro

%macro bn_check_twisted_ident
    SWAP2
    // stack: a   , b   , c   , d
    ISZERO
    SWAP3
    // stack: d   , b   , c   , a==0
    ISZERO
    SWAP2
    // stack: c   , b   , d==0, a==0
    ISZERO
    SWAP1
    // stack: b   , c==0, d==0, a==0
    ISZERO
    // stack: b==0, c==0, d==0, a==0
    MUL // Cheaper than AND
    MUL // Cheaper than AND
    MUL // Cheaper than AND
%endmacro
