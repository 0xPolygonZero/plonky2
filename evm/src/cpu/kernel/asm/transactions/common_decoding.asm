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
    DUP1 %mstore_kernel_general_2(0x4CC355)
    %decode_rlp_list_len
    %stack (pos, len) -> (len, len, pos, %%after)
    %jumpi(decode_and_store_access_list)
    // stack: len, pos, %%after
    POP SWAP1 POP
    // stack: pos
    %mload_kernel_general_2(0x4CC355) DUP2 SUB %mstore_kernel_general_2(0x4CC356)
 %%after:
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

global decode_and_store_access_list:
    // stack: len, pos
    DUP2 ADD
    // stack: end_pos, pos
    %mload_kernel_general_2(0x4CC355) DUP2 SUB %mstore_kernel_general_2(0x4CC356)
    SWAP1
global decode_and_store_access_list_loop:
    // stack: pos, end_pos
    DUP2 DUP2 EQ %jumpi(decode_and_store_access_list_finish)
    // stack: pos, end_pos
    %decode_rlp_list_len
    // stack: pos, internal_len, end_pos
    SWAP1 DUP2
    // stack: pos, internal_len, pos, end_pos
    ADD
    // stack: end_internal_pos, pos, end_pos // TODO: Don't need end_internal_pos
    SWAP1
    // stack: pos, end_internal_pos, end_pos
    %decode_rlp_scalar // TODO: Should panic when address is not 20 bytes?
    // stack: pos, addr, end_internal_pos, end_pos
    SWAP1
    // stack: addr, pos, end_internal_pos, end_pos
    DUP1 %insert_accessed_addresses_no_return
    // stack: addr, pos, end_internal_pos, end_pos
    %add_address_cost
    // stack: addr, pos, end_internal_pos, end_pos
    SWAP1
    // stack: pos, addr, end_internal_pos, end_pos
    %decode_rlp_list_len
    // stack: pos, sk_len, addr, end_internal_pos, end_pos
    SWAP1 DUP2 ADD
    // stack: sk_end_pos, pos, addr, end_internal_pos, end_pos
    SWAP1
    // stack: pos, sk_end_pos, addr, end_internal_pos, end_pos
global sk_loop:
    DUP2 DUP2 EQ %jumpi(end_sk)
    // stack: pos, sk_end_pos, addr, end_internal_pos, end_pos
    %decode_rlp_scalar // TODO: Should panic when key is not 32 bytes?
    %stack (pos, key, sk_end_pos, addr, end_internal_pos, end_pos) ->
        (addr, key, 0, pos, sk_end_pos, addr, end_internal_pos, end_pos)
    %insert_accessed_storage_keys_no_return
    // stack: pos, sk_end_pos, addr, end_internal_pos, end_pos
    %add_storage_key_cost
    %jump(sk_loop)
global end_sk:
    %stack (pos, sk_end_pos, addr, end_internal_pos, end_pos) -> (pos, end_pos)
    %jump(decode_and_store_access_list_loop)
global decode_and_store_access_list_finish:
    %stack (pos, end_pos, retdest) -> (retdest, pos)
    JUMP

%macro add_address_cost
    %mload_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_DATA_COST)
    %add_const(@GAS_ACCESSLISTADDRESS)
    %mstore_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_DATA_COST)
%endmacro

%macro add_storage_key_cost
    %mload_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_DATA_COST)
    %add_const(@GAS_ACCESSLISTSTORAGE)
    %mstore_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_DATA_COST)
%endmacro
