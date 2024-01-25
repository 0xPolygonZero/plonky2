// It is convenient to have a single panic routine, which we can jump to from
// anywhere.
global panic:
    PANIC

// Consumes the top element and asserts that it is zero.
%macro assert_zero
    %jumpi(panic)
%endmacro

%macro assert_zero(ret)
    %jumpi($ret)
%endmacro

// Consumes the top element and asserts that it is nonzero.
%macro assert_nonzero
    ISZERO
    %jumpi(panic)
%endmacro

%macro assert_nonzero(ret)
    ISZERO
    %jumpi($ret)
%endmacro

%macro assert_eq
    SUB
    %jumpi(panic)
%endmacro

%macro assert_eq(ret)
    SUB
    %jumpi($ret)
%endmacro

%macro assert_lt
    // %assert_zero is cheaper than %assert_nonzero, so we will leverage the
    // fact that (x < y) == !(x >= y).
    GE
    %assert_zero
%endmacro

%macro assert_lt(ret)
    GE
    %assert_zero($ret)
%endmacro

%macro assert_le
    // %assert_zero is cheaper than %assert_nonzero, so we will leverage the
    // fact that (x <= y) == !(x > y).
    GT
    %assert_zero
%endmacro

%macro assert_le(ret)
    GT
    %assert_zero($ret)
%endmacro

%macro assert_gt
    // %assert_zero is cheaper than %assert_nonzero, so we will leverage the
    // fact that (x > y) == !(x <= y).
    LE
    %assert_zero
%endmacro

%macro assert_gt(ret)
    LE
    %assert_zero($ret)
%endmacro

%macro assert_ge
    // %assert_zero is cheaper than %assert_nonzero, so we will leverage the
    // fact that (x >= y) == !(x < y).
    LT
    %assert_zero
%endmacro

%macro assert_ge(ret)
    LT
    %assert_zero($ret)
%endmacro

%macro assert_eq_const(c)
    PUSH $c
    SUB
    %jumpi(panic)
%endmacro

%macro assert_lt_const(c)
    // %assert_zero is cheaper than %assert_nonzero, so we will leverage the
    // fact that (x < c) == !(x >= c).
    %ge_const($c)
    %assert_zero
%endmacro

%macro assert_le_const(c)
    // %assert_zero is cheaper than %assert_nonzero, so we will leverage the
    // fact that (x <= c) == !(x > c).
    %gt_const($c)
    %assert_zero
%endmacro

%macro assert_gt_const(c)
    // %assert_zero is cheaper than %assert_nonzero, so we will leverage the
    // fact that (x > c) == !(x <= c).
    %le_const($c)
    %assert_zero
%endmacro

%macro assert_ge_const(c)
    // %assert_zero is cheaper than %assert_nonzero, so we will leverage the
    // fact that (x >= c) == !(x < c).
    %lt_const($c)
    %assert_zero
%endmacro
