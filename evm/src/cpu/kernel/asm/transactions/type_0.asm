// Type 0 transactions, aka legacy transaction, have the format
//     rlp([nonce, gas_price, gas_limit, destination, amount, data, v, r, s])
//
// The field v was originally encoded as
//     27 + y_parity
// but as of EIP 155 it can also be encoded as
//     35 + 2 * chain_id + y_parity
//
// If a chain_id is present in v, the signed data is
//     keccak256(rlp([nonce, gas_price, gas_limit, destination, amount, data, chain_id, 0, 0]))
// otherwise, it is
//     keccak256(rlp([nonce, gas_price, gas_limit, destination, amount, data]))

global process_type_0_txn:
    JUMPDEST
    // stack: (empty)
    PUSH process_txn_with_len
    PUSH 0 // initial pos
    // stack: pos, process_txn_with_len
    %jump(decode_rlp_list_len)

process_txn_with_len:
    // We don't actually need the length.
    %stack (pos, len) -> (pos)

    PUSH store_nonce
    SWAP1
    // stack: pos, store_nonce
    %jump(decode_rlp_scalar)

store_nonce:
    %stack (pos, nonce) -> (@TXN_FIELD_NONCE, nonce, pos)
    %mstore_current(@SEGMENT_NORMALIZED_TXN)

    // stack: pos
    PUSH store_gas_price
    SWAP1
    // stack: pos, store_gas_price
    %jump(decode_rlp_scalar)

store_gas_price:
    // For legacy transactions, we set both the
    // TXN_FIELD_MAX_PRIORITY_FEE_PER_GAS and TXN_FIELD_MAX_FEE_PER_GAS
    // fields to gas_price.
    %stack (pos, gas_price) -> (@TXN_FIELD_MAX_PRIORITY_FEE_PER_GAS, gas_price,
                                @TXN_FIELD_MAX_FEE_PER_GAS, gas_price, pos)
    %mstore_current(@SEGMENT_NORMALIZED_TXN)
    %mstore_current(@SEGMENT_NORMALIZED_TXN)

    // stack: pos
    PUSH store_gas_limit
    SWAP1
    // stack: pos, store_gas_limit
    %jump(decode_rlp_scalar)

store_gas_limit:
    %stack (pos, gas_limit) -> (@TXN_FIELD_GAS_LIMIT, gas_limit, pos)
    %mstore_current(@SEGMENT_NORMALIZED_TXN)

    // Peak at the RLP to see if the next byte is zero.
    // If so, there is no destination field, so skip the store_destination step.
    // stack: pos
    DUP1
    %mload_current(@SEGMENT_RLP_RAW)
    ISZERO
    // stack: destination_empty, pos
    %jumpi(parse_amount)

    // If we got here, there is a destination field.
    PUSH store_destination
    SWAP1
    // stack: pos, store_destination
    %jump(decode_rlp_scalar)

store_destination:
    %stack (pos, destination) -> (@TXN_FIELD_DESTINATION, destination, pos)
    %mstore_current(@SEGMENT_NORMALIZED_TXN)
    // stack: pos

parse_amount:
    // stack: pos
    PUSH store_amount
    SWAP1
    // stack: pos, store_amount
    %jump(decode_rlp_scalar)

store_amount:
    %stack (pos, amount) -> (@TXN_FIELD_AMOUNT, amount, pos)
    %mstore_current(@SEGMENT_NORMALIZED_TXN)

    // stack: pos
    PUSH store_data_len
    SWAP1
    // stack: pos, store_data_len
    %jump(decode_rlp_string_len)

store_data_len:
    %stack (pos, data_len) -> (@TXN_FIELD_DATA_LEN, data_len, pos, data_len, pos, data_len)
    %mstore_current(@SEGMENT_NORMALIZED_TXN)
    // stack: pos, data_len, pos, data_len
    ADD
    // stack: new_pos, pos, data_len

    // Memcpy the txn data from @SEGMENT_RLP_RAW to @SEGMENT_TXN_DATA.
    PUSH parse_v
    %stack (parse_v, new_pos, old_pos, data_len) -> (old_pos, data_len, parse_v, new_pos)
    PUSH @SEGMENT_RLP_RAW
    GET_CONTEXT
    PUSH 0
    PUSH @SEGMENT_TXN_DATA
    GET_CONTEXT
    // stack: DST, SRC, data_len, parse_v, new_pos
    %jump(memcpy)

parse_v:
    // stack: pos
    PUSH process_v
    SWAP1
    // stack: pos, process_v
    %jump(decode_rlp_scalar)

process_v:
    // stack: pos, v
    SWAP1
    // stack: v, pos
    DUP1
    %gt_const(28)
    // stack: v > 28, v, pos
    %jumpi(process_v_new_style)

    // We have an old style v, so y_parity = v - 27.
    // No chain ID is present, so we can leave TXN_FIELD_CHAIN_ID_PRESENT and
    // TXN_FIELD_CHAIN_ID with their default values of zero.
    // stack: v, pos
    %sub_const(27)
    %stack (y_parity, pos) -> (@TXN_FIELD_Y_PARITY, y_parity, pos)
    %mstore_current(@SEGMENT_NORMALIZED_TXN)

    // stack: pos
    %jump(parse_r)

process_v_new_style:
    // stack: v, pos
    // We have a new style v, so chain_id_present = 1,
    // chain_id = (v - 35) / 2, and y_parity = (v - 35) % 2.
    %stack (v, pos) -> (@TXN_FIELD_CHAIN_ID_PRESENT, 1, v, pos)
    %mstore_current(@SEGMENT_NORMALIZED_TXN)

    // stack: v, pos
    %sub_const(35)
    DUP1
    // stack: v - 35, v - 35, pos
    %div_const(2)
    // stack: chain_id, v - 35, pos
    PUSH @TXN_FIELD_CHAIN_ID
    %mstore_current(@SEGMENT_NORMALIZED_TXN)

    // stack: v - 35, pos
    %mod_const(2)
    // stack: y_parity, pos
    PUSH @TXN_FIELD_Y_PARITY
    %mstore_current(@SEGMENT_NORMALIZED_TXN)

parse_r:
    // stack: pos
    PUSH store_r
    SWAP1
    // stack: pos, store_r
    %jump(decode_rlp_scalar)

store_r:
    %stack (pos, r) -> (@TXN_FIELD_R, r, pos)
    %mstore_current(@SEGMENT_NORMALIZED_TXN)

    // stack: pos
    PUSH store_s
    SWAP1
    // stack: pos, store_s
    %jump(decode_rlp_scalar)

store_s:
    %stack (pos, s) -> (@TXN_FIELD_S, s)
    %mstore_current(@SEGMENT_NORMALIZED_TXN)
    // stack: (empty)

    // TODO: Write the signed txn data to memory, where it can be hashed and
    // checked against the signature.

    %jump(process_normalized_txn)
