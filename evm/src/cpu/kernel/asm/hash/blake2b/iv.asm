global blake2b_iv_const:
    // IV constants (big-endian)

    // IV_0
    BYTES 106, 9, 230, 103
    BYTES 243, 188, 201, 8

    // IV_1
    BYTES 187, 103, 174, 133
    BYTES 132, 202, 167, 59

    // IV_2
    BYTES 60, 110, 243, 114
    BYTES 254, 148, 248, 43

    // IV_3
    BYTES 165, 79, 245, 58
    BYTES 95, 29, 54, 241

    // IV_4
    BYTES 81, 14, 82, 127
    BYTES 173, 230, 130, 209

    // IV_5
    BYTES 155, 5, 104, 140
    BYTES 43, 62, 108, 31

    // IV_6
    BYTES 31, 131, 217, 171
    BYTES 251, 65, 189, 107

    // IV_7
    BYTES 91, 224, 205, 25
    BYTES 19, 126, 33, 121

%macro blake2b_iv
    // stack: i, ...
    PUSH blake2b_iv_const
    // stack: blake2b_iv_const, i, ...
    SWAP1
    // stack: i, blake2b_iv_const, ...
    %mul_const(8)
    ADD
    // stack: blake2b_iv_const + 2 * i, ...
    DUP1
    // stack: blake2b_iv_const + 2 * i, blake2b_iv_const + 2 * i, ...
    %add_const(4)
    // stack: blake2b_iv_const + 2 * i + 1, blake2b_iv_const + 2 * i, ...
    %mload_kernel_code_u32
    SWAP1
    %mload_kernel_code_u32
    // stack: IV_i[32:], IV_i[:32], ...
    %shl_const(32)
    // stack: IV_i[32:] << 32, IV_i[:32], ...
    OR
    // stack: IV_i, ...
%endmacro

%macro blake2b_iv_i(i)
    PUSH $i
    %blake2b_iv
%endmacro
