global intrinsic_gas:
    // stack: retdest
    // Calculate the number of zero and nonzero bytes in the txn data.
    PUSH 0 // zeros = 0
    PUSH 0 // i = 0

count_zeros_loop:
    // stack: i, zeros, retdest
    DUP1
    %mload_txn_field(@TXN_FIELD_DATA_LEN)
    EQ
    // stack: i == data.len, i, zeros, retdest
    %jumpi(count_zeros_finish)

    // stack: i, zeros, retdest
    DUP1
    %mload_kernel(@SEGMENT_TXN_DATA)
    ISZERO
    // stack: data[i] == 0, i, zeros
    %stack (data_i_is_zero, i, zeros) -> (data_i_is_zero, zeros, i)
    ADD
    // stack: zeros', i, retdest
    SWAP1
    // stack: i, zeros', retdest
    %increment
    // stack: i', zeros', retdest
    %jump(count_zeros_loop)

count_zeros_finish:
    // stack: i, zeros, retdest
    POP
    // stack: zeros, retdest
    DUP1
    // stack: zeros, zeros, retdest
    %mload_txn_field(@TXN_FIELD_DATA_LEN)
    // stack: data.len, zeros, zeros, retdest
    SUB
    // stack: nonzeros, zeros, retdest
    %mul_const(@GAS_TXDATANONZERO)
    // stack: gas_nonzeros, zeros, retdest
    SWAP1
    %mul_const(@GAS_TXDATAZERO)
    // stack: gas_zeros, gas_nonzeros, retdest
    ADD
    // stack: gas_txndata, retdest

    %is_contract_creation
    %mul_const(@GAS_TXCREATE)
    // stack: gas_creation, gas_txndata, retdest

    PUSH @GAS_TRANSACTION
    // stack: gas_txn, gas_creation, gas_txndata, retdest

    // TODO: Add num_access_list_addresses * GAS_ACCESSLISTADDRESS
    // TODO: Add num_access_list_slots * GAS_ACCESSLISTSTORAGE

    ADD
    ADD
    // stack: total_gas, retdest

    SWAP1
    JUMP
