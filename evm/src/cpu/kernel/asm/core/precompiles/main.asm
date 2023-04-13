%macro handle_precompiles
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
    PUSH %%after
    DUP5
    // stack: address, %%after, new_ctx, kexit_info, gas, address, value, args_offset, args_size, ret_offset, ret_size
    %jump(handle_precompiles)
%%after:
    // stack: new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size
%endmacro

global handle_precompiles:
    // stack: addr, retdest
    DUP1 %eq_const(@ECREC)  %jumpi(precompile_ecrec)
    DUP1 %eq_const(@SHA256) %jumpi(precompile_sha256)
    DUP1 %eq_const(@RIP160) %jumpi(precompile_rip160)
    DUP1 %eq_const(@ID)     %jumpi(precompile_id)
    DUP1 %eq_const(@EXPMOD) %jumpi(precompile_expmod)
    DUP1 %eq_const(@BN_ADD) %jumpi(precompile_bn_add)
    DUP1 %eq_const(@BN_MUL) %jumpi(precompile_bn_mul)
    DUP1 %eq_const(@SNARKV) %jumpi(precompile_snarkv)
    %eq_const(@BLAKE2_F) %jumpi(precompile_blake2_f)
    // stack: retdest
    JUMP

global pop_and_return_success:
    POP
    %leftover_gas
    // stack: leftover_gas
    PUSH 1 // success
    %jump(terminate_common)

%macro zero_out_kernel_general
    PUSH 0 PUSH 0 %mstore_kernel_general
    PUSH 0 PUSH 1 %mstore_kernel_general
    PUSH 0 PUSH 2 %mstore_kernel_general
    PUSH 0 PUSH 3 %mstore_kernel_general
    PUSH 0 PUSH 4 %mstore_kernel_general
    PUSH 0 PUSH 5 %mstore_kernel_general
    PUSH 0 PUSH 6 %mstore_kernel_general
    PUSH 0 PUSH 7 %mstore_kernel_general
    PUSH 0 PUSH 8 %mstore_kernel_general
    PUSH 0 PUSH 9 %mstore_kernel_general
    PUSH 0 PUSH 10 %mstore_kernel_general
    PUSH 0 PUSH 11 %mstore_kernel_general
    PUSH 0 PUSH 12 %mstore_kernel_general
    PUSH 0 PUSH 13 %mstore_kernel_general
    PUSH 0 PUSH 14 %mstore_kernel_general
    PUSH 0 PUSH 15 %mstore_kernel_general
    PUSH 0 PUSH 16 %mstore_kernel_general
    PUSH 0 PUSH 17 %mstore_kernel_general
    PUSH 0 PUSH 18 %mstore_kernel_general
    PUSH 0 PUSH 19 %mstore_kernel_general
    PUSH 0 PUSH 20 %mstore_kernel_general
    PUSH 0 PUSH 21 %mstore_kernel_general
    PUSH 0 PUSH 22 %mstore_kernel_general
    PUSH 0 PUSH 23 %mstore_kernel_general
    PUSH 0 PUSH 24 %mstore_kernel_general
    PUSH 0 PUSH 25 %mstore_kernel_general
    PUSH 0 PUSH 26 %mstore_kernel_general
    PUSH 0 PUSH 27 %mstore_kernel_general
    PUSH 0 PUSH 28 %mstore_kernel_general
    PUSH 0 PUSH 29 %mstore_kernel_general
    PUSH 0 PUSH 30 %mstore_kernel_general
    PUSH 0 PUSH 31 %mstore_kernel_general
    PUSH 0 PUSH 32 %mstore_kernel_general
    PUSH 0 PUSH 33 %mstore_kernel_general
    PUSH 0 PUSH 34 %mstore_kernel_general
    PUSH 0 PUSH 35 %mstore_kernel_general
    PUSH 0 PUSH 36 %mstore_kernel_general
    PUSH 0 PUSH 37 %mstore_kernel_general
    PUSH 0 PUSH 38 %mstore_kernel_general
    PUSH 0 PUSH 39 %mstore_kernel_general
    PUSH 0 PUSH 40 %mstore_kernel_general
    PUSH 0 PUSH 41 %mstore_kernel_general
    PUSH 0 PUSH 42 %mstore_kernel_general
    PUSH 0 PUSH 43 %mstore_kernel_general
    PUSH 0 PUSH 44 %mstore_kernel_general
    PUSH 0 PUSH 45 %mstore_kernel_general
    PUSH 0 PUSH 46 %mstore_kernel_general
    PUSH 0 PUSH 47 %mstore_kernel_general
    PUSH 0 PUSH 48 %mstore_kernel_general
    PUSH 0 PUSH 49 %mstore_kernel_general
    PUSH 0 PUSH 50 %mstore_kernel_general
    PUSH 0 PUSH 51 %mstore_kernel_general
    PUSH 0 PUSH 52 %mstore_kernel_general
    PUSH 0 PUSH 53 %mstore_kernel_general
    PUSH 0 PUSH 54 %mstore_kernel_general
    PUSH 0 PUSH 55 %mstore_kernel_general
    PUSH 0 PUSH 56 %mstore_kernel_general
    PUSH 0 PUSH 57 %mstore_kernel_general
    PUSH 0 PUSH 58 %mstore_kernel_general
    PUSH 0 PUSH 59 %mstore_kernel_general
    PUSH 0 PUSH 60 %mstore_kernel_general
    PUSH 0 PUSH 61 %mstore_kernel_general
    PUSH 0 PUSH 62 %mstore_kernel_general
    PUSH 0 PUSH 63 %mstore_kernel_general
    PUSH 0 PUSH 64 %mstore_kernel_general
    PUSH 0 PUSH 65 %mstore_kernel_general
    PUSH 0 PUSH 66 %mstore_kernel_general
    PUSH 0 PUSH 67 %mstore_kernel_general
    PUSH 0 PUSH 68 %mstore_kernel_general
    PUSH 0 PUSH 69 %mstore_kernel_general
    PUSH 0 PUSH 70 %mstore_kernel_general
    PUSH 0 PUSH 71 %mstore_kernel_general
    PUSH 0 PUSH 72 %mstore_kernel_general
    PUSH 0 PUSH 73 %mstore_kernel_general
    PUSH 0 PUSH 74 %mstore_kernel_general
    PUSH 0 PUSH 75 %mstore_kernel_general
    PUSH 0 PUSH 76 %mstore_kernel_general
    PUSH 0 PUSH 77 %mstore_kernel_general
    PUSH 0 PUSH 78 %mstore_kernel_general
    PUSH 0 PUSH 79 %mstore_kernel_general
    PUSH 0 PUSH 80 %mstore_kernel_general
    PUSH 0 PUSH 81 %mstore_kernel_general
    PUSH 0 PUSH 82 %mstore_kernel_general
    PUSH 0 PUSH 83 %mstore_kernel_general
    PUSH 0 PUSH 84 %mstore_kernel_general
    PUSH 0 PUSH 85 %mstore_kernel_general
    PUSH 0 PUSH 86 %mstore_kernel_general
    PUSH 0 PUSH 87 %mstore_kernel_general
    PUSH 0 PUSH 88 %mstore_kernel_general
    PUSH 0 PUSH 89 %mstore_kernel_general
    PUSH 0 PUSH 90 %mstore_kernel_general
    PUSH 0 PUSH 91 %mstore_kernel_general
    PUSH 0 PUSH 92 %mstore_kernel_general
    PUSH 0 PUSH 93 %mstore_kernel_general
    PUSH 0 PUSH 94 %mstore_kernel_general
    PUSH 0 PUSH 95 %mstore_kernel_general
    PUSH 0 PUSH 96 %mstore_kernel_general
    PUSH 0 PUSH 97 %mstore_kernel_general
    PUSH 0 PUSH 98 %mstore_kernel_general
    PUSH 0 PUSH 99 %mstore_kernel_general
%endmacro
