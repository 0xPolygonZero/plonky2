%macro handle_precompiles
    // stack: address, new_ctx, (old stack)
    PUSH %%after
    SWAP1
    // stack: address, %%after, new_ctx, (old stack)
    %jump(handle_precompiles)
%%after:
    // stack: new_ctx, (old stack)
%endmacro

global handle_precompiles:
    // stack: address, retdest, new_ctx, (old stack)
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
    // stack: _unused, kexit_info
    POP
    %leftover_gas
    // stack: leftover_gas
    PUSH 1 // success
    %jump(terminate_common)

global after_precompile:
    %mload_global_metadata(@GLOBAL_METADATA_IS_PRECOMPILE_FROM_EOA) %jumpi(process_message_txn_after_call)
    %stack (success, leftover_gas, new_ctx, kexit_info, callgas, address, value, args_offset, args_size, ret_offset, ret_size) ->
        (success, leftover_gas, new_ctx, kexit_info, ret_offset, ret_size)
    %jump(after_call_instruction)

%macro handle_precompiles_from_eoa
    // stack: retdest
    %mload_txn_field(@TXN_FIELD_TO)
    // stack: addr, retdest
    DUP1 %is_precompile
    %jumpi(handle_precompiles_from_eoa)
    // stack: addr, retdest
    POP
%endmacro

global handle_precompiles_from_eoa:
    PUSH 1 %mstore_global_metadata(@GLOBAL_METADATA_IS_PRECOMPILE_FROM_EOA)
    // stack: addr, retdest
    %create_context
    // stack: new_ctx, addr, retdest
    %non_intrinisic_gas %set_new_ctx_gas_limit
    // stack: new_ctx, addr, retdest

    // Set calldatasize and copy txn data to calldata.
    %mload_txn_field(@TXN_FIELD_DATA_LEN)
    %stack (calldata_size, new_ctx) -> (calldata_size, new_ctx, calldata_size)
    %set_new_ctx_calldata_size
    %stack (new_ctx, calldata_size) -> (@SEGMENT_TXN_DATA, @SEGMENT_CALLDATA, new_ctx, calldata_size, handle_precompiles_from_eoa_finish, new_ctx)
    SWAP2 %build_address_no_offset // DST
    // stack: DST, SRC, calldata_size, handle_precompiles_from_eoa_finish, new_ctx
    %jump(memcpy_bytes)

handle_precompiles_from_eoa_finish:
    %stack (new_ctx, addr, retdest) -> (addr, new_ctx, retdest)
    %handle_precompiles
    PANIC // We already checked that a precompile is called, so this should be unreachable.
