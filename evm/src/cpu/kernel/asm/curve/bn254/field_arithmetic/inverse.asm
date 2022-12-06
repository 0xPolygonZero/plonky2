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

global inverse_fp12:
    // stack:                           ptr, inv, retdest
    // DUP1  %load_fp12
    // stack:                        f, ptr, inv, retdest
    DUP14
    // stack:                   inv, f, ptr, inv, retdest 
    PROVER_INPUT(ff::bn254_base::inverse_fp12)
    PROVER_INPUT(ff::bn254_base::inverse_fp12)
    PROVER_INPUT(ff::bn254_base::inverse_fp12)
    PROVER_INPUT(ff::bn254_base::inverse_fp12)
    PROVER_INPUT(ff::bn254_base::inverse_fp12)
    PROVER_INPUT(ff::bn254_base::inverse_fp12)
    PROVER_INPUT(ff::bn254_base::inverse_fp12)
    PROVER_INPUT(ff::bn254_base::inverse_fp12)
    PROVER_INPUT(ff::bn254_base::inverse_fp12)
    PROVER_INPUT(ff::bn254_base::inverse_fp12)
    PROVER_INPUT(ff::bn254_base::inverse_fp12)
    PROVER_INPUT(ff::bn254_base::inverse_fp12)
    // stack:             f^-1, inv, f, ptr, inv, retdest
    DUP13
    // stack:        inv, f^-1, inv, f, ptr, inv, retdest
    // %store_fp12  POP
    // stack:                        f, ptr, inv, retdest
    %pop4  %pop4  %pop4
    // stack:                           ptr, inv, retdest 
    PUSH check_inv  PUSH 200
    // stack:           200, check_inv, ptr, inv, retdest 
    DUP4  DUP4
    // stack: ptr, inv, 200, check_inv, ptr, inv, retdest 
    %jump(mul_fp12)
global check_inv:
    // stack:                      200, ptr, inv, retdest
    // %eq_unit_fp12
    // stack:                  is_unit, ptr, inv, retdest
    %assert_nonzero
    // stack:                           ptr, inv, retdest
    POP  SWAP1  
    // stack:                                retdest, inv
    JUMP
