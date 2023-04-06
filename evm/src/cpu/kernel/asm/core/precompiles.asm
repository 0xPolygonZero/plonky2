%macro handle_precompiles
    // stack: address, gas, kexit_info, value, args_offset, args_size, ret_offset, ret_size
    PUSH %%after
    DUP2
    // stack: address, %%after, address, gas, kexit_info, value, args_offset, args_size, ret_offset, ret_size
    %jump(handle_precompiles)
%%after:
    // stack: (empty)
%endmacro

global handle_precompiles:
    // stack: addr, retdest
    DUP1 %eq_const(@ECREC)  %jumpi(ecrec)
    DUP1 %eq_const(@SHA256) %jumpi(sha256)
    DUP1 %eq_const(@RIP160) %jumpi(rip160)
    DUP1 %eq_const(@ID)     %jumpi(id)
    DUP1 %eq_const(@EXPMOD) %jumpi(expmod)
    DUP1 %eq_const(@BN_ADD) %jumpi(bn_add)
    DUP1 %eq_const(@BN_MUL) %jumpi(bn_mul)
    DUP1 %eq_const(@SNARKV) %jumpi(snarkv)
    %eq_const(@BLAKE2_F) %jumpi(blake2_f)
    // stack: retdest
    JUMP

ecrec:
    // stack: addr, retdest

sha256:
    %stack (address, retdest, address, gas, kexit_info, value, args_offset, args_size, ret_offset, ret_size) ->
        //(args_offset, args_size, ret_offset, ret_size, kexit_info)
        (args_size, kexit_info, args_offset, args_size, ret_offset, ret_size)

    %num_bytes_to_num_words
    // stack: data_words_len
    %mul_const(@SHA256_DYNAMIC_GAS)
    PUSH @SHA256_STATIC_GAS
    ADD
    %charge_gas
    %stack (kexit_info, args_offset, args_size, ret_offset, ret_size) ->
        (args_offset, args_size, ret_offset, ret_size, kexit_info)


    %zero_out_kernel_general

    GET_CONTEXT
    %stack (ctx, args_offset, args_size) ->
        (
        0, @SEGMENT_KERNEL_GENERAL, 1,              // DST
        ctx, @SEGMENT_MAIN_MEMORY, args_offset,     // SRC
        args_size, sha2,                            // count, retdest
        0, args_size, sha256_contd                  // sha2 input: virt, num_bytes, retdest
        )
    %jump(memcpy)

sha256_contd:
    // stack: hash
    GET_CONTEXT
    %stack (ctx, hash) -> (ctx, @SEGMENT_RETURNDATA, 0, hash, 32, sha256_contd_bis)
    %jump(mstore_unpacking)
global sha256_contd_bis:
    POP
    // stack: ret_offset, ret_size, kexit_info
    %jump(after_precompile)

    /*
    PUSH @SHA256_STATIC_GAS
    // stack: static_gas, retdest
    %mload_txn_field(@TXN_FIELD_DATA_LEN) // TODO: should be calldata len if this is used for an actual precompile call.
    // stack: data_bytes_len, static_gas, retdest
    %num_bytes_to_num_words
    // stack: data_words_len, static_gas, retdest
    %mul_const(@SHA256_DYNAMIC_GAS)
    // stack: dynamic_gas, static_gas, retdest
    ADD
    SWAP1 JUMP
    */

rip160:
    // stack: addr, retdest

id:
    // stack: addr, retdest

expmod:
    // stack: addr, retdest

// TODO: Check input and go to `fault_exception` if input is invalid.
bn_add:
    // stack: addr, retdest

// TODO: Check input and go to `fault_exception` if input is invalid.
bn_mul:
    // stack: addr, retdest

// TODO: Check input and go to `fault_exception` if input is invalid.
snarkv:
    // stack: addr, retdest

// TODO: Check input and go to `fault_exception` if input is invalid.
blake2_f:
    // stack: addr, retdest

/*
ecrec:
    // stack: addr, retdest
    POP
    PUSH @ECREC_GAS
    SWAP1 JUMP

sha256:
    // stack: addr, retdest
    %pop2
    // stack: (empty)
    %calldatasize
    // stack: calldata_size

    GET_CONTEXT
    %stack (ctx, calldata_size) ->
        (
        0, @SEGMENT_KERNEL_GENERAL, 1, // DST
        ctx, @SEGMENT_CALLDATA, 0,     // SRC
        calldata_size, sha2,           // count, retdest
        0, calldata_size, sha256_contd // sha2 input: virt, num_bytes, retdest
        )
    %jump(memcpy)

sha256_contd:
    GET_CONTEXT
    %stack (ctx, hash) -> (ctx, @SEGMENT_RETURNDATA, 0, hash, 32, sha256_contd_bis)
sha256_contd_bis:

    PUSH @SHA256_STATIC_GAS
    // stack: static_gas, retdest
    %mload_txn_field(@TXN_FIELD_DATA_LEN) // TODO: should be calldata len if this is used for an actual precompile call.
    // stack: data_bytes_len, static_gas, retdest
    %num_bytes_to_num_words
    // stack: data_words_len, static_gas, retdest
    %mul_const(@SHA256_DYNAMIC_GAS)
    // stack: dynamic_gas, static_gas, retdest
    ADD
    SWAP1 JUMP

rip160_gas:
    // stack: addr, retdest
    POP
    PUSH @RIP160_STATIC_GAS
    // stack: static_gas, retdest
    %mload_txn_field(@TXN_FIELD_DATA_LEN)
    // stack: data_bytes_len, static_gas, retdest
    %num_bytes_to_num_words
    // stack: data_words_len, static_gas, retdest
    %mul_const(@RIP160_DYNAMIC_GAS)
    // stack: dynamic_gas, static_gas, retdest
    ADD
    SWAP1 JUMP

id_gas:
    // stack: addr, retdest
    POP
    PUSH @ID_STATIC_GAS
    // stack: static_gas, retdest
    %mload_txn_field(@TXN_FIELD_DATA_LEN)
    // stack: data_bytes_len, static_gas, retdest
    %num_bytes_to_num_words
    // stack: data_words_len, static_gas, retdest
    %mul_const(@ID_DYNAMIC_GAS)
    // stack: dynamic_gas, static_gas, retdest
    ADD
    SWAP1 JUMP

expmod_gas:
    // stack: addr, retdest
    POP
    PUSH @EXPMOD_MIN_GAS // TODO: Complete this.
    SWAP1 JUMP

// TODO: Check input and go to `fault_exception` if input is invalid.
bn_add_gas:
    // stack: addr, retdest
    POP
    PUSH @BN_ADD_GAS
    SWAP1 JUMP

// TODO: Check input and go to `fault_exception` if input is invalid.
bn_mul_gas:
    // stack: addr, retdest
    POP
    PUSH @BN_MUL_GAS
    SWAP1 JUMP

// TODO: Check input and go to `fault_exception` if input is invalid.
snarkv_gas:
    // stack: addr, retdest
    POP
    %mload_txn_field(@TXN_FIELD_DATA_LEN)
    %div_const(192)
    // stack: k, retdest
    %mul_const(@SNARKV_DYNAMIC_GAS)
    %add_const(@SNARKV_STATIC_GAS)
    SWAP1 JUMP

// TODO: Check input and go to `fault_exception` if input is invalid.
blake2_f_gas:
    // stack: addr, retdest
    POP
    PUSH 3 %mload_kernel(@SEGMENT_TXN_DATA)
    PUSH 2 %mload_kernel(@SEGMENT_TXN_DATA)
    PUSH 1 %mload_kernel(@SEGMENT_TXN_DATA)
    PUSH 0 %mload_kernel(@SEGMENT_TXN_DATA)
    // stack: l0, l1, l2, l3, retdest
    %mul_const(0xff) ADD
    // stack: acc, l2, l3, retdest
    %mul_const(0xff) ADD
    // stack: acc, l3, retdest
    %mul_const(0xff) ADD
    SWAP1 JUMP
*/


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


global after_precompile:
    // stack: ret_offset, ret_size, kexit_info
    GET_CONTEXT
    %stack (ctx, ret_offset, ret_size) ->
        (
        ctx, @SEGMENT_MAIN_MEMORY, ret_offset,  // DST
        ctx, @SEGMENT_RETURNDATA, 0,            // SRC
        ret_size, after_precompile_finish       // count, retdest
        )
    %jump(memcpy)

after_precompile_finish:
    %stack (kexit_info) -> (kexit_info, 1) // success
    EXIT_KERNEL
