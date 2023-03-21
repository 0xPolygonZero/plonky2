// Type 0 transactions, aka legacy transaction, have the format
//     rlp([nonce, gas_price, gas_limit, to, value, data, v, r, s])
//
// The field v was originally encoded as
//     27 + y_parity
// but as of EIP 155 it can also be encoded as
//     35 + 2 * chain_id + y_parity
//
// If a chain_id is present in v, the signed data is
//     keccak256(rlp([nonce, gas_price, gas_limit, to, value, data, chain_id, 0, 0]))
// otherwise, it is
//     keccak256(rlp([nonce, gas_price, gas_limit, to, value, data]))

global process_type_0_txn:
    // stack: retdest
    PUSH 0 // initial pos
    // stack: pos, retdest
    %decode_rlp_list_len
    // We don't actually need the length.
    %stack (pos, len) -> (pos)

    // stack: pos, retdest
    %decode_and_store_nonce
    %decode_and_store_gas_price_legacy
    %decode_and_store_gas_limit
    %decode_and_store_to
    %decode_and_store_value
    %decode_and_store_data
    // stack: pos, retdest

    // Parse the "v" field.
    // stack: pos, retdest
    %decode_rlp_scalar
    // stack: pos, v, retdest
    SWAP1
    // stack: v, pos, retdest
    DUP1
    %gt_const(28)
    // stack: v > 28, v, pos, retdest
    %jumpi(process_v_new_style)

    // We have an old style v, so y_parity = v - 27.
    // No chain ID is present, so we can leave TXN_FIELD_CHAIN_ID_PRESENT and
    // TXN_FIELD_CHAIN_ID with their default values of zero.
    // stack: v, pos, retdest
    %sub_const(27)
    %stack (y_parity, pos) -> (y_parity, pos)
    %mstore_txn_field(@TXN_FIELD_Y_PARITY)

    // stack: pos, retdest
    %jump(decode_r_and_s)

process_v_new_style:
    // stack: v, pos, retdest
    // We have a new style v, so chain_id_present = 1,
    // chain_id = (v - 35) / 2, and y_parity = (v - 35) % 2.
    %stack (v, pos) -> (1, v, pos)
    %mstore_txn_field(@TXN_FIELD_CHAIN_ID_PRESENT)

    // stack: v, pos, retdest
    %sub_const(35)
    DUP1
    // stack: v - 35, v - 35, pos, retdest
    %div_const(2)
    // stack: chain_id, v - 35, pos, retdest
    %mstore_txn_field(@TXN_FIELD_CHAIN_ID)

    // stack: v - 35, pos, retdest
    %mod_const(2)
    // stack: y_parity, pos, retdest
    %mstore_txn_field(@TXN_FIELD_Y_PARITY)

decode_r_and_s:
    // stack: pos, retdest
    %decode_and_store_r
    %decode_and_store_s
    // stack: pos, retdest
    POP
    // stack: retdest

type_0_compute_signed_data:
    // If a chain_id is present in v, the signed data is
    //     keccak256(rlp([nonce, gas_price, gas_limit, to, value, data, chain_id, 0, 0]))
    // otherwise, it is
    //     keccak256(rlp([nonce, gas_price, gas_limit, to, value, data]))

    %alloc_rlp_block
    // stack: rlp_start, retdest
    %mload_txn_field(@TXN_FIELD_NONCE)
    // stack: nonce, rlp_start, retdest
    DUP2
    // stack: rlp_pos, nonce, rlp_start, retdest
    %encode_rlp_scalar
    // stack: rlp_pos, rlp_start, retdest

    %mload_txn_field(@TXN_FIELD_MAX_FEE_PER_GAS)
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos, rlp_start, retdest

    %mload_txn_field(@TXN_FIELD_GAS_LIMIT)
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos, rlp_start, retdest

    %mload_txn_field(@TXN_FIELD_TO)
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos, rlp_start, retdest

    %mload_txn_field(@TXN_FIELD_VALUE)
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos, rlp_start, retdest

    // Encode txn data.
    %mload_txn_field(@TXN_FIELD_DATA_LEN)
    PUSH 0 // ADDR.virt
    PUSH @SEGMENT_TXN_DATA
    PUSH 0 // ADDR.context
    // stack: ADDR: 3, len, rlp_pos, rlp_start, retdest
    PUSH after_serializing_txn_data
    // stack: after_serializing_txn_data, ADDR: 3, len, rlp_pos, rlp_start, retdest
    SWAP5
    // stack: rlp_pos, ADDR: 3, len, after_serializing_txn_data, rlp_start, retdest
    %jump(encode_rlp_string)

after_serializing_txn_data:
    // stack: rlp_pos, rlp_start, retdest
    %mload_txn_field(@TXN_FIELD_CHAIN_ID_PRESENT)
    ISZERO %jumpi(finish_rlp_list)
    // stack: rlp_pos, rlp_start, retdest

    %mload_txn_field(@TXN_FIELD_CHAIN_ID)
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos, rlp_start, retdest

    PUSH 0
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos, rlp_start, retdest

    PUSH 0
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos, rlp_start, retdest

finish_rlp_list:
    %prepend_rlp_list_prefix
    // stack: prefix_start_pos, rlp_len, retdest
    PUSH @SEGMENT_RLP_RAW
    PUSH 0 // context
    // stack: ADDR: 3, rlp_len, retdest
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
