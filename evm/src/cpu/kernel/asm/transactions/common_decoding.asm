// Store chain ID = 1. Used for non-legacy txns which always have a chain ID.
%macro store_chain_id_present_true
    PUSH 1
    %mstore_txn_field(@TXN_FIELD_CHAIN_ID_PRESENT)
%endmacro

// Decode the chain ID and store it.
%macro decode_and_store_chain_id
    // stack: rlp_addr
    %decode_rlp_scalar
    %stack (rlp_addr, chain_id) -> (chain_id, rlp_addr)
    %mstore_txn_field(@TXN_FIELD_CHAIN_ID)
    // stack: rlp_addr
%endmacro

// Decode the nonce and store it.
%macro decode_and_store_nonce
    // stack: rlp_addr
    %decode_rlp_scalar
    %stack (rlp_addr, nonce) -> (nonce, rlp_addr)
    %mstore_txn_field(@TXN_FIELD_NONCE)
    // stack: rlp_addr
%endmacro

// Decode the gas price and, since this is for legacy txns, store it as both
// TXN_FIELD_MAX_PRIORITY_FEE_PER_GAS and TXN_FIELD_MAX_FEE_PER_GAS.
%macro decode_and_store_gas_price_legacy
    // stack: rlp_addr
    %decode_rlp_scalar
    %stack (rlp_addr, gas_price) -> (gas_price, gas_price, rlp_addr)
    %mstore_txn_field(@TXN_FIELD_MAX_PRIORITY_FEE_PER_GAS)
    %mstore_txn_field(@TXN_FIELD_MAX_FEE_PER_GAS)
    // stack: rlp_addr
%endmacro

// Decode the max priority fee and store it.
%macro decode_and_store_max_priority_fee
    // stack: rlp_addr
    %decode_rlp_scalar
    %stack (rlp_addr, gas_price) -> (gas_price, rlp_addr)
    %mstore_txn_field(@TXN_FIELD_MAX_PRIORITY_FEE_PER_GAS)
    // stack: rlp_addr
%endmacro

// Decode the max fee and store it.
%macro decode_and_store_max_fee
    // stack: rlp_addr
    %decode_rlp_scalar
    %stack (rlp_addr, gas_price) -> (gas_price, rlp_addr)
    %mstore_txn_field(@TXN_FIELD_MAX_FEE_PER_GAS)
    // stack: rlp_addr
%endmacro

// Decode the gas limit and store it.
%macro decode_and_store_gas_limit
    // stack: rlp_addr
    %decode_rlp_scalar
    %stack (rlp_addr, gas_limit) -> (gas_limit, rlp_addr)
    %mstore_txn_field(@TXN_FIELD_GAS_LIMIT)
    // stack: rlp_addr
%endmacro

// Decode the "to" field and store it.
// This field is either 160-bit or empty in the case of a contract creation txn.
%macro decode_and_store_to
    // stack: rlp_addr
    %decode_rlp_string_len
    // stack: rlp_addr, len
    SWAP1
    // stack: len, rlp_addr
    DUP1 ISZERO %jumpi(%%contract_creation)
    // stack: len, rlp_addr
    DUP1 %eq_const(20) ISZERO %jumpi(invalid_txn) // Address is 160-bit
    %stack (len, rlp_addr) -> (rlp_addr, len, %%with_scalar)
    %jump(decode_int_given_len)
%%with_scalar:
    // stack: rlp_addr, int
    SWAP1
    %mstore_txn_field(@TXN_FIELD_TO)
    // stack: rlp_addr
    %jump(%%end)
%%contract_creation:
    // stack: len, rlp_addr
    POP
    PUSH 1 %mstore_global_metadata(@GLOBAL_METADATA_CONTRACT_CREATION)
    // stack: rlp_addr
%%end:
%endmacro

// Decode the "value" field and store it.
%macro decode_and_store_value
    // stack: rlp_addr
    %decode_rlp_scalar
    %stack (rlp_addr, value) -> (value, rlp_addr)
    %mstore_txn_field(@TXN_FIELD_VALUE)
    // stack: rlp_addr
%endmacro

// Decode the calldata field, store its length in @TXN_FIELD_DATA_LEN, and copy it to @SEGMENT_TXN_DATA.
%macro decode_and_store_data
    // stack: rlp_addr
    // Decode the data length, store it, and compute new_rlp_addr after any data.
    %decode_rlp_string_len
    %stack (rlp_addr, data_len) -> (data_len, rlp_addr, data_len, rlp_addr, data_len)
    %mstore_txn_field(@TXN_FIELD_DATA_LEN)
    // stack: rlp_addr, data_len, rlp_addr, data_len
    ADD
    // stack: new_rlp_addr, old_rlp_addr, data_len

    // Memcpy the txn data from @SEGMENT_RLP_RAW to @SEGMENT_TXN_DATA.
    %stack (new_rlp_addr, old_rlp_addr, data_len) -> (old_rlp_addr, data_len, %%after, new_rlp_addr)
    // old_rlp_addr has context 0. We will call GET_CONTEXT and update it.
    GET_CONTEXT ADD
    PUSH @SEGMENT_TXN_DATA
    GET_CONTEXT ADD
    // stack: DST, SRC, data_len, %%after, new_rlp_addr
    %jump(memcpy_bytes)

%%after:
    // stack: new_rlp_addr
%endmacro

%macro decode_and_store_access_list
    // stack: rlp_addr
    DUP1 %mstore_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_RLP_START)
    %decode_rlp_list_len
    %stack (rlp_addr, len) -> (len, len, rlp_addr, %%after)
    %jumpi(decode_and_store_access_list)
    // stack: len, rlp_addr, %%after
    POP SWAP1 POP
    // stack: rlp_addr
    %mload_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_RLP_START) DUP2 SUB %mstore_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_RLP_LEN)
%%after:
%endmacro

%macro decode_and_store_y_parity
    // stack: rlp_addr
    %decode_rlp_scalar
    %stack (rlp_addr, y_parity) -> (y_parity, rlp_addr)
    %mstore_txn_field(@TXN_FIELD_Y_PARITY)
    // stack: rlp_addr
%endmacro

%macro decode_and_store_r
    // stack: rlp_addr
    %decode_rlp_scalar
    %stack (rlp_addr, r) -> (r, rlp_addr)
    %mstore_txn_field(@TXN_FIELD_R)
    // stack: rlp_addr
%endmacro

%macro decode_and_store_s
    // stack: rlp_addr
    %decode_rlp_scalar
    %stack (rlp_addr, s) -> (s, rlp_addr)
    %mstore_txn_field(@TXN_FIELD_S)
    // stack: rlp_addr
%endmacro


// The access list is of the form `[[{20 bytes}, [{32 bytes}...]]...]`.
global decode_and_store_access_list:
    // stack: len, rlp_addr
    DUP2 ADD
    // stack: end_rlp_addr, rlp_addr
    // Store the RLP length.
    %mload_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_RLP_START) DUP2 SUB %mstore_global_metadata(@GLOBAL_METADATA_ACCESS_LIST_RLP_LEN)
    SWAP1
decode_and_store_access_list_loop:
    // stack: rlp_addr, end_rlp_addr
    DUP2 DUP2 EQ %jumpi(decode_and_store_access_list_finish)
    // stack: rlp_addr, end_rlp_addr
    %decode_rlp_list_len // Should be a list `[{20 bytes}, [{32 bytes}...]]`
    // stack: rlp_addr, internal_len, end_rlp_addr
    SWAP1 POP // We don't need the length of this list.
    // stack: rlp_addr, end_rlp_addr
    %decode_rlp_scalar // Address // TODO: Should panic when address is not 20 bytes?
    // stack: rlp_addr, addr, end_rlp_addr
    SWAP1
    // stack: addr, rlp_addr, end_rlp_addr
    DUP1 %insert_accessed_addresses_no_return
    // stack: addr, rlp_addr, end_rlp_addr
    %add_address_cost
    // stack: addr, rlp_addr, end_rlp_addr
    SWAP1
    // stack: rlp_addr, addr, end_rlp_addr
    %decode_rlp_list_len // Should be a list of storage keys `[{32 bytes}...]`
    // stack: rlp_addr, sk_len, addr, end_rlp_addr
    SWAP1 DUP2 ADD
    // stack: sk_end_rlp_addr, rlp_addr, addr, end_rlp_addr
    SWAP1
    // stack: rlp_addr, sk_end_rlp_addr, addr, end_rlp_addr
sk_loop:
    DUP2 DUP2 EQ %jumpi(end_sk)
    // stack: rlp_addr, sk_end_rlp_addr, addr, end_rlp_addr
    %decode_rlp_scalar // Storage key // TODO: Should panic when key is not 32 bytes?
    %stack (rlp_addr, key, sk_end_rlp_addr, addr, end_rlp_addr) ->
        (addr, key, sk_loop_contd, rlp_addr, sk_end_rlp_addr, addr, end_rlp_addr)
    %jump(insert_accessed_storage_keys_with_original_value)
sk_loop_contd:
    // stack: rlp_addr, sk_end_rlp_addr, addr, end_rlp_addr
    %add_storage_key_cost
    %jump(sk_loop)
end_sk:
    %stack (rlp_addr, sk_end_rlp_addr, addr, end_rlp_addr) -> (rlp_addr, end_rlp_addr)
    %jump(decode_and_store_access_list_loop)
decode_and_store_access_list_finish:
    %stack (rlp_addr, end_rlp_addr, retdest) -> (retdest, rlp_addr)
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
