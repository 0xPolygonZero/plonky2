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


global test_inverse_fp12:
    // stack:                        ptr, f, ptr, inv, retdest
    %store_fp12
    // stack:                                ptr, inv, retdest
    %jump(inverse_fp12)

global inverse_fp12:
    // stack:                 ptr, inv, retdest
    DUP1  %load_fp12
    // stack:              f, ptr, inv, retdest
    DUP14
    // stack:         inv, f, ptr, inv, retdest
    %prover_inv_fp12
    // stack:   f^-1, inv, f, ptr, inv, retdest
    DUP13  %store_fp12
    // stack:         inv, f, ptr, inv, retdest
    POP  %pop4  %pop4  %pop4
    // stack:                 ptr, inv, retdest
    PUSH 100  PUSH check_inv
    // stack: check_inv, 100, ptr, inv, retdest 
    SWAP3  SWAP1  SWAP2
    // stack: ptr, inv, 100, check_inv, retdest 
    %jump(mul_fp12)
global check_inv:
    // stack:        retdest
    PUSH 100  %load_fp12
    // stack: unit?, retdest
    %assert_eq_unit_fp12
    // stack:        retdest
    JUMP

%macro prover_inv_fp12
    PROVER_INPUT(ffe::bn254_base::ext_inv11)
    PROVER_INPUT(ffe::bn254_base::ext_inv10)
    PROVER_INPUT(ffe::bn254_base::ext_inv9)
    PROVER_INPUT(ffe::bn254_base::ext_inv8)
    PROVER_INPUT(ffe::bn254_base::ext_inv7)
    PROVER_INPUT(ffe::bn254_base::ext_inv6)
    PROVER_INPUT(ffe::bn254_base::ext_inv5)
    PROVER_INPUT(ffe::bn254_base::ext_inv4)
    PROVER_INPUT(ffe::bn254_base::ext_inv3)
    PROVER_INPUT(ffe::bn254_base::ext_inv2)
    PROVER_INPUT(ffe::bn254_base::ext_inv1)
    PROVER_INPUT(ffe::bn254_base::ext_inv0)
%endmacro

%macro assert_eq_unit_fp12
    %assert_eq_const(1)
    %assert_eq_const(0)
    %assert_eq_const(0)
    %assert_eq_const(0)
    %assert_eq_const(0)
    %assert_eq_const(0)
    %assert_eq_const(0)
    %assert_eq_const(0)
    %assert_eq_const(0)
    %assert_eq_const(0)
    %assert_eq_const(0)
    %assert_eq_const(0)
%endmacro
