// Store chain ID = 1. Used for non-legacy txns which always have a chain ID.
%macro store_chain_id_present_true
    PUSH 1
    %mstore_txn_field(@TXN_FIELD_CHAIN_ID_PRESENT)
%endmacro

// Decode the chain ID and store it.
%macro decode_and_store_chain_id
    // stack: pos
    %decode_rlp_scalar
    %stack (pos, chain_id) -> (chain_id, pos)
    %mstore_txn_field(@TXN_FIELD_CHAIN_ID)
    // stack: pos
%endmacro

// Decode the nonce and store it.
%macro decode_and_store_nonce
    // stack: pos
    %decode_rlp_scalar
    %stack (pos, nonce) -> (nonce, pos)
    %mstore_txn_field(@TXN_FIELD_NONCE)
    // stack: pos
%endmacro

// Decode the gas price and, since this is for legacy txns, store it as both
// TXN_FIELD_MAX_PRIORITY_FEE_PER_GAS and TXN_FIELD_MAX_FEE_PER_GAS.
%macro decode_and_store_gas_price_legacy
    // stack: pos
    %decode_rlp_scalar
    %stack (pos, gas_price) -> (gas_price, gas_price, pos)
    %mstore_txn_field(@TXN_FIELD_MAX_PRIORITY_FEE_PER_GAS)
    %mstore_txn_field(@TXN_FIELD_MAX_FEE_PER_GAS)
    // stack: pos
%endmacro

// Decode the max priority fee and store it.
%macro decode_and_store_max_priority_fee
    // stack: pos
    %decode_rlp_scalar
    %stack (pos, gas_price) -> (gas_price, pos)
    %mstore_txn_field(@TXN_FIELD_MAX_PRIORITY_FEE_PER_GAS)
    // stack: pos
%endmacro

// Decode the max fee and store it.
%macro decode_and_store_max_fee
    // stack: pos
    %decode_rlp_scalar
    %stack (pos, gas_price) -> (gas_price, pos)
    %mstore_txn_field(@TXN_FIELD_MAX_FEE_PER_GAS)
    // stack: pos
%endmacro

// Decode the gas limit and store it.
%macro decode_and_store_gas_limit
    // stack: pos
    %decode_rlp_scalar
    %stack (pos, gas_limit) -> (gas_limit, pos)
    %mstore_txn_field(@TXN_FIELD_GAS_LIMIT)
    // stack: pos
%endmacro

// Decode the "to" field and store it.
%macro decode_and_store_to
    // stack: pos
    %decode_rlp_scalar
    %stack (pos, to) -> (to, pos)
    %mstore_txn_field(@TXN_FIELD_TO)
    // stack: pos
%endmacro

// Decode the "value" field and store it.
%macro decode_and_store_value
    // stack: pos
    %decode_rlp_scalar
    %stack (pos, value) -> (value, pos)
    %mstore_txn_field(@TXN_FIELD_VALUE)
    // stack: pos
%endmacro

// Decode the calldata field, store its length in @TXN_FIELD_DATA_LEN, and copy it to @SEGMENT_TXN_DATA.
%macro decode_and_store_data
    // stack: pos
    // Decode the data length, store it, and compute new_pos after any data.
    %decode_rlp_string_len
    %stack (pos, data_len) -> (data_len, pos, data_len, pos, data_len)
    %mstore_txn_field(@TXN_FIELD_DATA_LEN)
    // stack: pos, data_len, pos, data_len
    ADD
    // stack: new_pos, old_pos, data_len

    // Memcpy the txn data from @SEGMENT_RLP_RAW to @SEGMENT_TXN_DATA.
    %stack (new_pos, old_pos, data_len) -> (old_pos, data_len, %%after, new_pos)
    PUSH @SEGMENT_RLP_RAW
    GET_CONTEXT
    PUSH 0
    PUSH @SEGMENT_TXN_DATA
    GET_CONTEXT
    // stack: DST, SRC, data_len, %%after, new_pos
    %jump(memcpy)

%%after:
    // stack: new_pos
%endmacro

%macro decode_and_store_access_list
    // stack: pos
    %decode_rlp_list_len
    %stack (pos, len) -> (len, pos)
    %jumpi(todo_access_lists_not_supported_yet)
    // stack: pos
%endmacro

%macro decode_and_store_y_parity
    // stack: pos
    %decode_rlp_scalar
    %stack (pos, y_parity) -> (y_parity, pos)
    %mstore_txn_field(@TXN_FIELD_Y_PARITY)
    // stack: pos
%endmacro

%macro decode_and_store_r
    // stack: pos
    %decode_rlp_scalar
    %stack (pos, r) -> (r, pos)
    %mstore_txn_field(@TXN_FIELD_R)
    // stack: pos
%endmacro

%macro decode_and_store_s
    // stack: pos
    %decode_rlp_scalar
    %stack (pos, s) -> (s, pos)
    %mstore_txn_field(@TXN_FIELD_S)
    // stack: pos
%endmacro

global todo_access_lists_not_supported_yet:
    PANIC
