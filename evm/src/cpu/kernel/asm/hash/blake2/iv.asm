global blake2_iv_const:
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

global blake2_iv:
    // stack: i, retdest
    PUSH blake2_iv_const
    // stack: blake2_iv_const, i, retdest
    SWAP1
    // stack: i, blake2_iv_const, retdest
    %mul_const(8)
    ADD
    // stack: blake2_iv_const + 2 * i, retdest
    DUP1
    // stack: blake2_iv_const + 2 * i, blake2_iv_const + 2 * i, retdest
    %add_const(4)
    // stack: blake2_iv_const + 2 * i + 1, blake2_iv_const + 2 * i, retdest
    %mload_kernel_code_u32
    SWAP1
    %mload_kernel_code_u32
    // stack: IV_i[32:], IV_i[:32], retdest
    %shl_const(32)
    // stack: IV_i[32:] << 32, IV_i[:32], retdest
    ADD // OR
    // stack: IV_i, retdest
    SWAP1
    JUMP

%macro blake2_iv
    %stack (i) -> (i, %%after)
    %jump(blake2_iv)
%%after:
%endmacro

// Load the initial hash value (the IV, but with params XOR'd into the first word).
global blake2_initial_hash_value:
    // stack: retdest
    PUSH 8
    // stack: i=8, retdest
blake2_initial_hash_loop:
    // stack: i, IV_i, ..., IV_7, retdest
    %decrement
    // stack: i-1, IV_i, ..., IV_7, retdest
    PUSH blake2_initial_hash_return
    // stack: blake2_initial_hash_return, i-1, IV_i, ..., IV_7, retdest
    DUP2
    // stack: i-1, blake2_initial_hash_return, i-1, IV_i, ..., IV_7, retdest
    %jump(blake2_iv)
blake2_initial_hash_return:
    // stack: IV_(i-1), i-1, IV_i, ..., IV_7, retdest
    SWAP1
    // stack: i-1, IV_(i-1), IV_i, ..., IV_7, retdest
    DUP1
    // stack: i-1, i-1, IV_(i-1), ..., IV_7, retdest
    %jumpi(blake2_initial_hash_loop)
    // stack: i-1=0, IV_0, ..., IV_7, retdest
    POP
    // stack: IV_0, ..., IV_7, retdest
    PUSH 0x01010040 // params: key = 00, digest_size = 64 = 0x40
    XOR
    // stack: IV_0 ^ params, IV_1, IV_2, IV_3, IV_4, IV_5, IV_6, IV_7, retdest
    %stack(iv: 8, ret) -> (ret, iv)
    JUMP

