/// Division modulo the BN254 prime

// Returns y * (x^-1) where the inverse is taken modulo N
%macro divfp254
    // stack: x   , y
    %inverse
    // stack: x^-1, y
    MULFP254
%endmacro

// Non-deterministically provide the inverse modulo N.
%macro inverse
    // stack:        x
    PROVER_INPUT(ff::bn254_base::inverse)
    // stack: x^-1 , x
    SWAP1  DUP2
    // stack: x^-1 , x, x^-1
    MULFP254
    // stack: x^-1 * x, x^-1
    %assert_eq_const(1)
    // stack:           x^-1
%endmacro


global inv_fp12:
    // stack:                ptr, inv, retdest
    DUP1  %load_fp12
    // stack:             f, ptr, inv, retdest
    DUP14
    // stack:        inv, f, ptr, inv, retdest
    %prover_inv_fp12
    // stack:  f^-1, inv, f, ptr, inv, retdest
    DUP13  %store_fp12
    // stack:        inv, f, ptr, inv, retdest
    POP  %pop4  %pop4  %pop4
    // stack:                ptr, inv, retdest
    PUSH 50  PUSH check_inv
    // stack: check_inv, 50, ptr, inv, retdest 
    %stack (check_inv, 50, ptr, inv) -> (ptr, inv, 50, check_inv)
    // stack: ptr, inv, 50, check_inv, retdest 
    %jump(mul_fp12)
global check_inv:
    // stack:        retdest
    PUSH 50  %load_fp12
    // stack: unit?, retdest
    %assert_eq_unit_fp12
    // stack:        retdest
    JUMP

%macro prover_inv_fp12
    PROVER_INPUT(ffe::bn254_base::component_11)
    PROVER_INPUT(ffe::bn254_base::component_10)
    PROVER_INPUT(ffe::bn254_base::component_9)
    PROVER_INPUT(ffe::bn254_base::component_8)
    PROVER_INPUT(ffe::bn254_base::component_7)
    PROVER_INPUT(ffe::bn254_base::component_6)
    PROVER_INPUT(ffe::bn254_base::component_5)
    PROVER_INPUT(ffe::bn254_base::component_4)
    PROVER_INPUT(ffe::bn254_base::component_3)
    PROVER_INPUT(ffe::bn254_base::component_2)
    PROVER_INPUT(ffe::bn254_base::component_1)
    PROVER_INPUT(ffe::bn254_base::component_0)
%endmacro

%macro assert_eq_unit_fp12
    %assert_eq_const(1)
    %assert_zero
    %assert_zero
    %assert_zero
    %assert_zero
    %assert_zero
    %assert_zero
    %assert_zero
    %assert_zero
    %assert_zero
    %assert_zero
    %assert_zero
%endmacro
