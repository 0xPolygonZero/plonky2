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
    PUSH 1 // initial pos, skipping over the 0x02 byte
    // stack: pos, retdest
    %decode_rlp_list_len
    // We don't actually need the length.
    %stack (pos, len) -> (pos)

    // stack: pos, retdest
    %store_chain_id_present_true
    %decode_and_store_chain_id
    %decode_and_store_nonce
    %decode_and_store_max_priority_fee
    %decode_and_store_max_fee
    %decode_and_store_gas_limit
    %decode_and_store_to
    %decode_and_store_value
    %decode_and_store_data
    %decode_and_store_access_list
    %decode_and_store_y_parity
    %decode_and_store_r
    %decode_and_store_s

    // stack: pos, retdest
    POP
    // stack: retdest

    // TODO: Check signature.

    %jump(process_normalized_txn)
