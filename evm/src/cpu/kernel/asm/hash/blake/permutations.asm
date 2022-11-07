permutation_1_constants:
    BYTES 14
    BYTES 10
    BYTES 4
    BYTES 8
    BYTES 9
    BYTES 15
    BYTES 13
    BYTES 6
    BYTES 1
    BYTES 12
    BYTES 0
    BYTES 2
    BYTES 11
    BYTES 7
    BYTES 5
    BYTES 3

permutation_2_constants:
    BYTES 11
    BYTES 8
    BYTES 12
    BYTES 0
    BYTES 5
    BYTES 2
    BYTES 15
    BYTES 13
    BYTES 10
    BYTES 14
    BYTES 3
    BYTES 6
    BYTES 7
    BYTES 1
    BYTES 9
    BYTES 4

permutation_3_constants:
    BYTES 7
    BYTES 9
    BYTES 3
    BYTES 1
    BYTES 13
    BYTES 12
    BYTES 11
    BYTES 14
    BYTES 2
    BYTES 6
    BYTES 5
    BYTES 10
    BYTES 4
    BYTES 0
    BYTES 15
    BYTES 8

permutation_4_constants:
    BYTES 9
    BYTES 0
    BYTES 5
    BYTES 7
    BYTES 2
    BYTES 4
    BYTES 10
    BYTES 15
    BYTES 14
    BYTES 1
    BYTES 11
    BYTES 12
    BYTES 6
    BYTES 8
    BYTES 3
    BYTES 13

permutation_5_constants:
    BYTES 2
    BYTES 12
    BYTES 6
    BYTES 10
    BYTES 0
    BYTES 11
    BYTES 8
    BYTES 3
    BYTES 4
    BYTES 13
    BYTES 7
    BYTES 5
    BYTES 15
    BYTES 14
    BYTES 1
    BYTES 9

permutation_6_constants:
    BYTES 12
    BYTES 5
    BYTES 1
    BYTES 15
    BYTES 14
    BYTES 13
    BYTES 4
    BYTES 10
    BYTES 0
    BYTES 7
    BYTES 6
    BYTES 3
    BYTES 9
    BYTES 2
    BYTES 8
    BYTES 11

permutation_7_constants:
    BYTES 13
    BYTES 11
    BYTES 7
    BYTES 14
    BYTES 12
    BYTES 1
    BYTES 3
    BYTES 9
    BYTES 5
    BYTES 0
    BYTES 15
    BYTES 4
    BYTES 8
    BYTES 6
    BYTES 2
    BYTES 10

permutation_8_constants:
    BYTES 6
    BYTES 15
    BYTES 14
    BYTES 9
    BYTES 11
    BYTES 3
    BYTES 0
    BYTES 8
    BYTES 12
    BYTES 2
    BYTES 13
    BYTES 7
    BYTES 1
    BYTES 4
    BYTES 10
    BYTES 5

permutation_9_constants:
    BYTES 10
    BYTES 2
    BYTES 8
    BYTES 4
    BYTES 7
    BYTES 6
    BYTES 1
    BYTES 5
    BYTES 15
    BYTES 11
    BYTES 9
    BYTES 14
    BYTES 3
    BYTES 12
    BYTES 13
    BYTES 0

%macro blake_permutation
    // stack: round, i
    PUSH permutation_1_constants
    // stack: permutation_1_constants, round, i
    SWAP1
    // stack: round, permutation_1_constants, i
    %mod_const(10)
    %mul_const(16)
    ADD
    %add_const($i)
    %mload_kernel_code
%endmacro
