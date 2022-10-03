// Type 1 transactions, introduced by EIP 2930, have the format
//     0x01 || rlp([chain_id, nonce, gas_price, gas_limit, to, value, data,
//                  access_list, y_parity, r, s])
//
// The signed data is
//     keccak256(0x01 || rlp([chain_id, nonce, gas_price, gas_limit, to, value,
//                            data, access_list]))

global process_type_1_txn:
    // stack: retdest
    PANIC // TODO: Unfinished
