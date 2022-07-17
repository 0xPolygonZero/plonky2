// It is convenient to have a single panic routine, which we can jump to from
// anywhere.
global panic:
    JUMPDEST
    PANIC

// Consumes the top element and asserts that it is zero.
%macro assert_zero
    %jumpi panic
%endmacro

// Consumes the top element and asserts that it is nonzero.
%macro assert_nonzero
    ISZERO
    %jumpi panic
%endmacro

%macro assert_eq
    EQ
    %assert_nonzero
%endmacro

%macro assert_lt
    LT
    %assert_nonzero
%endmacro

%macro assert_le
    LE
    %assert_nonzero
%endmacro

%macro assert_gt
    GT
    %assert_nonzero
%endmacro

%macro assert_ge
    GE
    %assert_nonzero
%endmacro

%macro assert_eq_const(c)
    %eq_const(c)
    %assert_nonzero
%endmacro

%macro assert_lt_const(c)
    // %assert_zero is cheaper than %assert_nonzero, so we will leverage the
    // fact that (x < c) == !(x >= c).
    %ge_const(c)
    %assert_zero
%endmacro

%macro assert_le_const(c)
    // %assert_zero is cheaper than %assert_nonzero, so we will leverage the
    // fact that (x <= c) == !(x > c).
    %gt_const(c)
    %assert_zero
%endmacro

%macro assert_gt_const(c)
    // %assert_zero is cheaper than %assert_nonzero, so we will leverage the
    // fact that (x > c) == !(x <= c).
    %le_const(c)
    %assert_zero
%endmacro

%macro assert_ge_const(c)
    // %assert_zero is cheaper than %assert_nonzero, so we will leverage the
    // fact that (x >= c) == !(x < c).
    %lt_const(c)
    %assert_zero
%endmacro
