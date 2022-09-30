// Type 2 transactions, introduced by EIP 1559, have the format
//     0x02 || rlp([chain_id, nonce, max_priority_fee_per_gas, max_fee_per_gas,
//                  gas_limit, to, value, data, access_list, y_parity, r, s])
//
// The signed data is
//     keccak256(0x02 || rlp([chain_id, nonce, max_priority_fee_per_gas,
//                            max_fee_per_gas, gas_limit, to, value, data,
//                            access_list]))

global process_type_2_txn:
    // stack: retdest
    PANIC // TODO: Unfinished
