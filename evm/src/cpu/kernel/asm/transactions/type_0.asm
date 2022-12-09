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

    // Decode the nonce and store it.
    // stack: pos, retdest
    %decode_rlp_scalar
    %stack (pos, nonce) -> (nonce, pos)
    %mstore_txn_field(@TXN_FIELD_NONCE)

    // Decode the gas price and store it.
    // For legacy transactions, we set both the
    // TXN_FIELD_MAX_PRIORITY_FEE_PER_GAS and TXN_FIELD_MAX_FEE_PER_GAS
    // fields to gas_price.
    // stack: pos, retdest
    %decode_rlp_scalar
    %stack (pos, gas_price) -> (gas_price, gas_price, pos)
    %mstore_txn_field(@TXN_FIELD_MAX_PRIORITY_FEE_PER_GAS)
    %mstore_txn_field(@TXN_FIELD_MAX_FEE_PER_GAS)

    // Decode the gas limit and store it.
    // stack: pos, retdest
    %decode_rlp_scalar
    %stack (pos, gas_limit) -> (gas_limit, pos)
    %mstore_txn_field(@TXN_FIELD_GAS_LIMIT)

    // Decode the "to" field and store it.
    // stack: pos, retdest
    %decode_rlp_scalar
    %stack (pos, to) -> (to, pos)
    %mstore_txn_field(@TXN_FIELD_TO)

    // Decode the value field and store it.
    // stack: pos, retdest
    %decode_rlp_scalar
    %stack (pos, value) -> (value, pos)
    %mstore_txn_field(@TXN_FIELD_VALUE)

    // Decode the data length, store it, and compute new_pos after any data.
    // stack: pos, retdest
    %decode_rlp_string_len
    %stack (pos, data_len) -> (data_len, pos, data_len, pos, data_len)
    %mstore_txn_field(@TXN_FIELD_DATA_LEN)
    // stack: pos, data_len, pos, data_len, retdest
    ADD
    // stack: new_pos, pos, data_len, retdest

    // Memcpy the txn data from @SEGMENT_RLP_RAW to @SEGMENT_TXN_DATA.
    PUSH parse_v
    %stack (parse_v, new_pos, old_pos, data_len) -> (old_pos, data_len, parse_v, new_pos)
    PUSH @SEGMENT_RLP_RAW
    GET_CONTEXT
    PUSH 0
    PUSH @SEGMENT_TXN_DATA
    GET_CONTEXT
    // stack: DST, SRC, data_len, parse_v, new_pos, retdest
    %jump(memcpy)

parse_v:
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
    %jump(parse_r)

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

parse_r:
    // stack: pos, retdest
    %decode_rlp_scalar
    %stack (pos, r) -> (r, pos)
    %mstore_txn_field(@TXN_FIELD_R)

    // stack: pos, retdest
    %decode_rlp_scalar
    %stack (pos, s) -> (s)
    %mstore_txn_field(@TXN_FIELD_S)
    // stack: retdest

type_0_compute_signed_data:
    // If a chain_id is present in v, the signed data is
    //     keccak256(rlp([nonce, gas_price, gas_limit, to, value, data, chain_id, 0, 0]))
    // otherwise, it is
    //     keccak256(rlp([nonce, gas_price, gas_limit, to, value, data]))

    %mload_txn_field(@TXN_FIELD_NONCE)
    // stack: nonce, retdest
    PUSH 9 // We start at 9 to leave room to prepend the largest possible RLP list header.
    // stack: rlp_pos, nonce, retdest
    %encode_rlp_scalar
    // stack: rlp_pos, retdest

    %mload_txn_field(@TXN_FIELD_MAX_FEE_PER_GAS)
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos, retdest

    %mload_txn_field(@TXN_FIELD_GAS_LIMIT)
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos, retdest

    %mload_txn_field(@TXN_FIELD_TO)
    SWAP1 %encode_rlp_160
    // stack: rlp_pos, retdest

    %mload_txn_field(@TXN_FIELD_VALUE)
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos, retdest

    // Encode txn data.
    %mload_txn_field(@TXN_FIELD_DATA_LEN)
    PUSH 0 // ADDR.virt
    PUSH @SEGMENT_TXN_DATA
    PUSH 0 // ADDR.context
    // stack: ADDR: 3, len, rlp_pos, retdest
    PUSH after_serializing_txn_data
    // stack: after_serializing_txn_data, ADDR: 3, len, rlp_pos, retdest
    SWAP5
    // stack: rlp_pos, ADDR: 3, len, after_serializing_txn_data, retdest
    %jump(encode_rlp_string)

after_serializing_txn_data:
    // stack: rlp_pos, retdest
    %mload_txn_field(@TXN_FIELD_CHAIN_ID_PRESENT)
    ISZERO %jumpi(finish_rlp_list)
    // stack: rlp_pos, retdest

    %mload_txn_field(@TXN_FIELD_CHAIN_ID)
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos, retdest

    PUSH 0
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos, retdest

    PUSH 0
    SWAP1 %encode_rlp_scalar
    // stack: rlp_pos, retdest

finish_rlp_list:
    %prepend_rlp_list_prefix
    // stack: start_pos, rlp_len, retdest
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
