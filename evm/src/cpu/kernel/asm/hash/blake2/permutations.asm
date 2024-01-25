global permutation_0_constants:
    BYTES 0, 1, 2, 3
    BYTES 4, 5, 6, 7
    BYTES 8, 9, 10, 11
    BYTES 12, 13, 14, 15

global permutation_1_constants:
    BYTES 14, 10, 4, 8
    BYTES 9, 15, 13, 6
    BYTES 1, 12, 0, 2
    BYTES 11, 7, 5, 3

global permutation_2_constants:
    BYTES 11, 8, 12, 0
    BYTES 5, 2, 15, 13
    BYTES 10, 14, 3, 6
    BYTES 7, 1, 9, 4

global permutation_3_constants:
    BYTES 7, 9, 3, 1
    BYTES 13, 12, 11, 14
    BYTES 2, 6, 5, 10
    BYTES 4, 0, 15, 8

global permutation_4_constants:
    BYTES 9, 0, 5, 7
    BYTES 2, 4, 10, 15
    BYTES 14, 1, 11, 12
    BYTES 6, 8, 3, 13

global permutation_5_constants:
    BYTES 2, 12, 6, 10
    BYTES 0, 11, 8, 3
    BYTES 4, 13, 7, 5
    BYTES 15, 14, 1, 9

global permutation_6_constants:
    BYTES 12, 5, 1, 15
    BYTES 14, 13, 4, 10
    BYTES 0, 7, 6, 3
    BYTES 9, 2, 8, 11

global permutation_7_constants:
    BYTES 13, 11, 7, 14
    BYTES 12, 1, 3, 9
    BYTES 5, 0, 15, 4
    BYTES 8, 6, 2, 10

global permutation_8_constants:
    BYTES 6, 15, 14, 9
    BYTES 11, 3, 0, 8
    BYTES 12, 2, 13, 7
    BYTES 1, 4, 10, 5

global permutation_9_constants:
    BYTES 10, 2, 8, 4
    BYTES 7, 6, 1, 5
    BYTES 15, 11, 9, 14
    BYTES 3, 12, 13, 0

global blake2_permutation:
    // stack: i, round, retdest
    PUSH permutation_0_constants
    // stack: permutation_0_constants, i, round, retdest
    SWAP2
    // stack: round, i, permutation_0_constants, retdest
    %mod_const(10)
    // stack: round % 10, i, permutation_0_constants, retdest
    %mul_const(16)
    ADD
    ADD
    %mload_kernel_code
    // stack: permutation_(round%10)_constants[i], retdest
    SWAP1
    JUMP

%macro blake2_permutation
    // stack: round, i
    PUSH %%after
    // stack: %%after, round, i
    SWAP2
    // stack: i, round, %%after
    %jump(blake2_permutation)
%%after:
%endmacro
