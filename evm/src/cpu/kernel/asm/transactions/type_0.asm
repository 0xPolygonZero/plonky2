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

    // TODO: Write the signed txn data to memory, where it can be hashed and
    // checked against the signature.

    %jump(process_normalized_txn)
