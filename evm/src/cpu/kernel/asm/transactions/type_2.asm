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
    // Initial rlp address offset of 1 (skipping over the 0x02 byte)
    PUSH 1
    PUSH @SEGMENT_RLP_RAW
    %build_kernel_address
    // stack: rlp_addr, retdest
    %decode_rlp_list_len
    // We don't actually need the length.
    %stack (rlp_addr, len) -> (rlp_addr)

    // stack: rlp_addr, retdest
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

    // stack: rlp_addr, retdest
    POP
    // stack: retdest

// From EIP-1559:
// The signature_y_parity, signature_r, signature_s elements of this transaction represent a secp256k1 signature over
// keccak256(0x02 || rlp([chain_id, nonce, max_priority_fee_per_gas, max_fee_per_gas, gas_limit, destination, amount, data, access_list]))
type_2_compute_signed_data:
    %alloc_rlp_block
    // stack: rlp_addr_start, retdest
    %mload_txn_field(@TXN_FIELD_CHAIN_ID)
    // stack: chain_id, rlp_start, retdest
    DUP2
    // stack: rlp_addr, chain_id, rlp_start, retdest
    %encode_rlp_scalar
    // stack: rlp_addr, rlp_start, retdest

    %mload_txn_field(@TXN_FIELD_NONCE)
    %encode_rlp_scalar_swapped_inputs
    // stack: rlp_addr, rlp_start, retdest

    %mload_txn_field(@TXN_FIELD_MAX_PRIORITY_FEE_PER_GAS)
    %encode_rlp_scalar_swapped_inputs
    // stack: rlp_addr, rlp_start, retdest

    %mload_txn_field(@TXN_FIELD_MAX_FEE_PER_GAS)
    %encode_rlp_scalar_swapped_inputs
    // stack: rlp_addr, rlp_start, retdest

    %mload_txn_field(@TXN_FIELD_GAS_LIMIT)
    %encode_rlp_scalar_swapped_inputs
    // stack: rlp_addr, rlp_start, retdest

    %mload_txn_field(@TXN_FIELD_TO)
    %mload_global_metadata(@GLOBAL_METADATA_CONTRACT_CREATION) %jumpi(zero_to)
    // stack: to, rlp_addr, rlp_start, retdest
    SWAP1 %encode_rlp_160
    %jump(after_to)
zero_to:
    // stack: to, rlp_addr, rlp_start, retdest
    %encode_rlp_scalar_swapped_inputs
    // stack: rlp_addr, rlp_start, retdest

after_to:
    %mload_txn_field(@TXN_FIELD_VALUE)
    %encode_rlp_scalar_swapped_inputs
    // stack: rlp_addr, rlp_start, retdest

    // Encode txn data.
    %mload_txn_field(@TXN_FIELD_DATA_LEN)
    PUSH @SEGMENT_TXN_DATA // ctx == virt == 0
    // stack: ADDR, len, rlp_addr, rlp_start, retdest
    PUSH after_serializing_txn_data
    // stack: after_serializing_txn_data, ADDR, len, rlp_addr, rlp_start, retdest
    SWAP3
    // stack: rlp_addr, ADDR, len, after_serializing_txn_data, rlp_start, retdest
    %jump(encode_rlp_string)

after_serializing_txn_data:
    // Instead of manually encoding the access list, we just copy the raw RLP from the transaction.
    %mload_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_RLP_START)
    %mload_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_RLP_LEN)
    %stack (al_len, al_start, rlp_addr, rlp_start, retdest) ->
        (
            rlp_addr,
            al_start,
            al_len,
            after_serializing_access_list,
            rlp_addr, rlp_start, retdest)
    %jump(memcpy_bytes)
after_serializing_access_list:
    // stack: rlp_addr, rlp_start, retdest
    %mload_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_RLP_LEN) ADD
    // stack: rlp_addr, rlp_start, retdest
    %prepend_rlp_list_prefix
    // stack: prefix_start_pos, rlp_len, retdest

    // Store a `2` in front of the RLP
    %decrement
    %stack (rlp_addr) -> (2, rlp_addr, rlp_addr)
    MSTORE_GENERAL
    // stack: rlp_addr, rlp_len, retdest

    // Hash the RLP + the leading `2`
    SWAP1 %increment SWAP1
    // stack: ADDR, len, retdest
    KECCAK_GENERAL
    // stack: hash, retdest

    %mload_txn_field(@TXN_FIELD_S)
    %mload_txn_field(@TXN_FIELD_R)
    %mload_txn_field(@TXN_FIELD_Y_PARITY) %add_const(27) // ecrecover interprets v as y_parity + 27

    PUSH store_origin
    // stack: store_origin, v, r, s, hash, retdest
    SWAP4
    // stack: hash, v, r, s, store_origin, retdest
    %jump(ecrecover)

store_origin:
    // stack: address, retdest
    // If ecrecover returned u256::MAX, that indicates failure.
    DUP1
    %eq_const(0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff)
    %jumpi(panic)

    // stack: address, retdest
    %mstore_txn_field(@TXN_FIELD_ORIGIN)
    // stack: retdest
    %jump(process_normalized_txn)
