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
// This field is either 160-bit or empty in the case of a contract creation txn.
%macro decode_and_store_to
    // stack: pos
    %decode_rlp_string_len
    // stack: pos, len
    SWAP1
    // stack: len, pos
    DUP1 ISZERO %jumpi(%%contract_creation)
    // stack: len, pos
    DUP1 %eq_const(20) ISZERO %jumpi(invalid_txn) // Address is 160-bit
    %stack (len, pos) -> (pos, len, %%with_scalar)
    %jump(decode_int_given_len)
%%with_scalar:
    // stack: pos, int
    SWAP1
    %mstore_txn_field(@TXN_FIELD_TO)
    // stack: pos
    %jump(%%end)
%%contract_creation:
    // stack: len, pos
    POP
    PUSH 1 %mstore_global_metadata(@GLOBAL_METADATA_CONTRACT_CREATION)
    // stack: pos
%%end:
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
    DUP1 %mstore_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_RLP_START)
    %decode_rlp_list_len
    %stack (pos, len) -> (len, len, pos, %%after)
    %jumpi(decode_and_store_access_list)
    // stack: len, pos, %%after
    POP SWAP1 POP
    // stack: pos
    %mload_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_RLP_START) DUP2 SUB %mstore_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_RLP_LEN)
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


// The access list is of the form `[[{20 bytes}, [{32 bytes}...]]...]`.
global decode_and_store_access_list:
    // stack: len, pos
    DUP2 ADD
    // stack: end_pos, pos
    // Store the RLP length.
    %mload_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_RLP_START) DUP2 SUB %mstore_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_RLP_LEN)
    SWAP1
decode_and_store_access_list_loop:
    // stack: pos, end_pos
    DUP2 DUP2 EQ %jumpi(decode_and_store_access_list_finish)
    // stack: pos, end_pos
    %decode_rlp_list_len // Should be a list `[{20 bytes}, [{32 bytes}...]]`
    // stack: pos, internal_len, end_pos
    SWAP1 POP // We don't need the length of this list.
    // stack: pos, end_pos
    %decode_rlp_scalar // Address // TODO: Should panic when address is not 20 bytes?
    // stack: pos, addr, end_pos
    SWAP1
    // stack: addr, pos, end_pos
    DUP1 %insert_accessed_addresses_no_return
    // stack: addr, pos, end_pos
    %add_address_cost
    // stack: addr, pos, end_pos
    SWAP1
    // stack: pos, addr, end_pos
    %decode_rlp_list_len // Should be a list of storage keys `[{32 bytes}...]`
    // stack: pos, sk_len, addr, end_pos
    SWAP1 DUP2 ADD
    // stack: sk_end_pos, pos, addr, end_pos
    SWAP1
    // stack: pos, sk_end_pos, addr, end_pos
sk_loop:
    DUP2 DUP2 EQ %jumpi(end_sk)
    // stack: pos, sk_end_pos, addr, end_pos
    %decode_rlp_scalar // Storage key // TODO: Should panic when key is not 32 bytes?
    %stack (pos, key, sk_end_pos, addr, end_pos) ->
        (addr, key, sk_loop_contd, pos, sk_end_pos, addr, end_pos)
    %jump(insert_accessed_storage_keys_with_original_value)
sk_loop_contd:
    // stack: pos, sk_end_pos, addr, end_pos
    %add_storage_key_cost
    %jump(sk_loop)
end_sk:
    %stack (pos, sk_end_pos, addr, end_pos) -> (pos, end_pos)
    %jump(decode_and_store_access_list_loop)
decode_and_store_access_list_finish:
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

insert_accessed_storage_keys_with_original_value:
    %stack (addr, key, retdest) -> (key, addr, after_read, addr, key, retdest)
    %jump(sload_with_addr)
after_read:
    %stack (value, addr, key, retdest) -> ( addr, key, value, retdest)
    %insert_accessed_storage_keys
    %pop2
    JUMP


sload_with_addr:
    %stack (slot, addr) -> (slot, addr, after_storage_read)
    %slot_to_storage_key
    // stack: storage_key, addr, after_storage_read
    PUSH 64 // storage_key has 64 nibbles
    %stack (n64, storage_key, addr, after_storage_read) -> (addr, n64, storage_key, after_storage_read)
    %mpt_read_state_trie
    // stack: account_ptr, 64, storage_key, after_storage_read
    DUP1 ISZERO %jumpi(ret_zero) // TODO: Fix this. This should never happen.
    // stack: account_ptr, 64, storage_key, after_storage_read
    %add_const(2)
    // stack: storage_root_ptr_ptr
    %mload_trie_data
    // stack: storage_root_ptr, 64, storage_key, after_storage_read
    %jump(mpt_read)

ret_zero:
    // stack: account_ptr, 64, storage_key, after_storage_read, retdest
    %pop4
    PUSH 0 SWAP1 JUMP
