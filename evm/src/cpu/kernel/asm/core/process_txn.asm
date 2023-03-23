// After the transaction data has been parsed into a normalized set of fields
// (see NormalizedTxnField), this routine processes the transaction.

// TODO: Save checkpoints in @CTX_METADATA_STATE_TRIE_CHECKPOINT_PTR and @SEGMENT_STORAGE_TRIE_CHECKPOINT_PTRS.

// Pre stack: retdest
// Post stack: (empty)
global process_normalized_txn:
    // stack: retdest
    %compute_fees
    // stack: retdest

    // Compute this transaction's intrinsic gas and store it.
    %intrinsic_gas
    %mstore_txn_field(@TXN_FIELD_INTRINSIC_GAS)
    // stack: retdest

    // Assert gas_limit >= intrinsic_gas.
    %mload_txn_field(@TXN_FIELD_INTRINSIC_GAS)
    %mload_txn_field(@TXN_FIELD_GAS_LIMIT)
    %assert_ge

    // TODO: Check that txn nonce matches account nonce.
    // TODO: Assert nonce is correct.
    // TODO: Assert sender has no code.
    // TODO: Assert sender balance >= gas_limit * gas_price + value.
    // TODO: Assert chain ID matches block metadata?
    // stack: retdest

global buy_gas:
    %mload_txn_field(@TXN_FIELD_COMPUTED_FEE_PER_GAS)
    %mload_txn_field(@TXN_FIELD_GAS_LIMIT)
    MUL
    // stack: gas_cost, retdest
    %mload_txn_field(@TXN_FIELD_ORIGIN)
    // stack: sender_addr, gas_cost, retdest
    %deduct_eth
    // stack: deduct_eth_status, retdest
    %jumpi(panic)
    // stack: retdest

global increment_sender_nonce:
    %mload_txn_field(@TXN_FIELD_ORIGIN)
    %increment_nonce

global process_based_on_type:
    %is_contract_creation
    %jumpi(process_contract_creation_txn)
    %jump(process_message_txn)

global process_contract_creation_txn:
    // stack: retdest

    %mload_txn_field(@TXN_FIELD_ORIGIN)
    // stack: origin, retdest
    DUP1 %nonce
    // stack: origin_nonce, origin, retdest
    SWAP1
    // stack: origin, origin_nonce, retdest
    %get_create_address
    // stack: address, retdest

    // Deduct value from caller.
    %mload_txn_field(@TXN_FIELD_VALUE)
    %mload_txn_field(@TXN_FIELD_ORIGIN)
    %deduct_eth
    // stack: deduct_eth_status, address, retdest
    %jumpi(panic)
    // stack: address, retdest

    // Write the new account's data to MPT data, and get a pointer to it.
    %get_trie_data_size
    // stack: account_ptr, address, retdest
    PUSH 1 %append_to_trie_data // nonce = 1
    // stack: account_ptr, address, retdest
    DUP2 %balance %mload_txn_field(@TXN_FIELD_VALUE) ADD %append_to_trie_data // balance = old_balance + txn_value
    // stack: account_ptr, address, retdest
    PUSH 0 %append_to_trie_data // storage_root = nil
    // stack: account_ptr, address, retdest
    PUSH @EMPTY_STRING_HASH %append_to_trie_data // code_hash = keccak('')
    // stack: account_ptr, address, retdest
    DUP2
    // stack: address, account_ptr, address, retdest
    %addr_to_state_key
    // stack: state_key, account_ptr, address, retdest
    %mpt_insert_state_trie
    // stack: address, retdest

    %create_context
    // stack: new_ctx, address, retdest

    // Copy the code from txdata to the new context's code segment.
    PUSH process_contract_creation_txn_after_code_loaded
    %mload_txn_field(@TXN_FIELD_DATA_LEN)
    PUSH 0 // SRC.offset
    PUSH @SEGMENT_TXN_DATA // SRC.segment
    PUSH 0 // SRC.context
    PUSH 0 // DST.offset
    PUSH @SEGMENT_CODE // DST.segment
    DUP7 // DST.context = new_ctx
    %jump(memcpy)

process_contract_creation_txn_after_code_loaded:
    // stack: new_ctx, address, retdest

    // Each line in the block below does not change the stack.
    DUP2 %set_new_ctx_addr
    %mload_txn_field(@TXN_FIELD_ORIGIN) %set_new_ctx_caller
    %mload_txn_field(@TXN_FIELD_VALUE) %set_new_ctx_value
    %set_new_ctx_parent_ctx
    %set_new_ctx_parent_pc(process_contract_creation_txn_after_constructor)
    %non_intrinisic_gas %set_new_ctx_gas_limit
    // stack: new_ctx, address, retdest

    %enter_new_ctx
    // (Old context) stack: new_ctx, address, retdest

global process_contract_creation_txn_after_constructor:
    // stack: success, leftover_gas, new_ctx, address, retdest
    POP // TODO: Success will go into the receipt when we support that.
    // stack: leftover_gas, new_ctx, address, retdest
    %pay_coinbase_and_refund_sender
    // stack: new_ctx, address, retdest
    POP
    POP
    JUMP

global process_message_txn:
    // stack: retdest
    %mload_txn_field(@TXN_FIELD_VALUE)
    %mload_txn_field(@TXN_FIELD_TO)
    DUP1 %insert_accessed_addresses
    %mload_txn_field(@TXN_FIELD_ORIGIN)
    DUP1 %insert_accessed_addresses
    // stack: from, to, amount, retdest
    %transfer_eth
    // stack: transfer_eth_status, retdest
    %jumpi(process_message_txn_insufficient_balance)
    // stack: retdest

    // Add precompiles to accessed addresses.
    PUSH @ECREC %insert_accessed_addresses
    PUSH @SHA256 %insert_accessed_addresses
    PUSH @RIP160 %insert_accessed_addresses
    PUSH @ID %insert_accessed_addresses
    PUSH @EXPMOD %insert_accessed_addresses
    PUSH @BN_ADD %insert_accessed_addresses
    PUSH @BN_MUL %insert_accessed_addresses
    PUSH @SNARKV %insert_accessed_addresses
    PUSH @BLAKE2_F %insert_accessed_addresses
    // TODO: Handle precompiles.

    // If to's code is empty, return.
    %mload_txn_field(@TXN_FIELD_TO) %ext_code_empty
    // stack: code_empty, retdest
    %jumpi(process_message_txn_return)

    // Otherwise, load to's code and execute it in a new context.
    // stack: retdest
    %create_context
    // stack: new_ctx, retdest
    PUSH process_message_txn_code_loaded
    PUSH @SEGMENT_CODE
    DUP3 // new_ctx
    %mload_txn_field(@TXN_FIELD_TO)
    // stack: address, new_ctx, segment, process_message_txn_code_loaded, new_ctx, retdest
    %jump(load_code)

global process_message_txn_insufficient_balance:
    // stack: retdest
    PANIC // TODO

global process_message_txn_return:
    // stack: retdest
    // Since no code was executed, the leftover gas is the non-intrinsic gas.
    %non_intrinisic_gas
    // stack: leftover_gas, retdest
    %pay_coinbase_and_refund_sender
    // stack: retdest
    JUMP

global process_message_txn_code_loaded:
    // stack: code_size, new_ctx, retdest
    %set_new_ctx_code_size
    // stack: new_ctx, retdest

    // Each line in the block below does not change the stack.
    %mload_txn_field(@TXN_FIELD_TO) %set_new_ctx_addr
    %mload_txn_field(@TXN_FIELD_ORIGIN) %set_new_ctx_caller
    %mload_txn_field(@TXN_FIELD_VALUE) %set_new_ctx_value
    %set_new_ctx_parent_ctx
    %set_new_ctx_parent_pc(process_message_txn_after_call)
    %non_intrinisic_gas %set_new_ctx_gas_limit
    // stack: new_ctx, retdest

    // TODO: Copy TXN_DATA to CALLDATA

    %enter_new_ctx
    // (Old context) stack: new_ctx, retdest

global process_message_txn_after_call:
    // stack: success, leftover_gas, new_ctx, retdest
    POP // TODO: Success will go into the receipt when we support that.
    // stack: leftover_gas, new_ctx, retdest
    %pay_coinbase_and_refund_sender
    // stack: new_ctx, retdest
    POP
    JUMP

%macro pay_coinbase_and_refund_sender
    // stack: leftover_gas
    DUP1
    // stack: leftover_gas, leftover_gas
    %mload_txn_field(@TXN_FIELD_GAS_LIMIT)
    SUB
    // stack: used_gas, leftover_gas
    %mload_global_metadata(@GLOBAL_METADATA_REFUND_COUNTER)
    // stack: refund, used_gas, leftover_gas
    DUP2 %div_const(2) // max_refund = used_gas/2
    // stack: max_refund, refund, used_gas, leftover_gas
    %min
    %stack (refund, used_gas, leftover_gas) -> (leftover_gas, refund, refund, used_gas)
    ADD
    // stack: leftover_gas', refund, used_gas
    SWAP2
    // stack: used_gas, refund, leftover_gas'
    SUB
    // stack: used_gas', leftover_gas'

    // Pay the coinbase.
    %mload_txn_field(@TXN_FIELD_COMPUTED_PRIORITY_FEE_PER_GAS)
    MUL
    // stack: used_gas_tip, leftover_gas'
    %mload_global_metadata(@GLOBAL_METADATA_BLOCK_BENEFICIARY)
    // stack: coinbase, used_gas_tip, leftover_gas'
    %add_eth
    // stack: leftover_gas'

    // Refund gas to the origin.
    %mload_txn_field(@TXN_FIELD_COMPUTED_FEE_PER_GAS)
    MUL
    // stack: leftover_gas_cost
    %mload_txn_field(@TXN_FIELD_ORIGIN)
    // stack: origin, leftover_gas_cost
    %add_eth
    // stack: (empty)
%endmacro

// Sets @TXN_FIELD_MAX_FEE_PER_GAS and @TXN_FIELD_MAX_PRIORITY_FEE_PER_GAS.
%macro compute_fees
    // stack: (empty)
    %mload_global_metadata(@GLOBAL_METADATA_BLOCK_BASE_FEE)
    %mload_txn_field(@TXN_FIELD_MAX_PRIORITY_FEE_PER_GAS)
    %mload_txn_field(@TXN_FIELD_MAX_FEE_PER_GAS)
    // stack: max_fee, max_priority_fee, base_fee
    DUP3 DUP2 %assert_ge // Assert max_fee >= base_fee
    // stack: max_fee, max_priority_fee, base_fee
    %stack (max_fee, max_priority_fee, base_fee) -> (max_fee, base_fee, max_priority_fee, base_fee)
    SUB
    // stack: max_fee - base_fee, max_priority_fee, base_fee
    %min
    // stack: computed_priority_fee, base_fee
    %stack (computed_priority_fee, base_fee) -> (computed_priority_fee, base_fee, computed_priority_fee)
    ADD
    // stack: computed_fee, computed_priority_fee
    %mstore_txn_field(@TXN_FIELD_COMPUTED_FEE_PER_GAS)
    %mstore_txn_field(@TXN_FIELD_COMPUTED_PRIORITY_FEE_PER_GAS)
    // stack: (empty)
%endmacro

%macro non_intrinisic_gas
    // stack: (empty)
    %mload_txn_field(@TXN_FIELD_INTRINSIC_GAS)
    %mload_txn_field(@TXN_FIELD_GAS_LIMIT)
    SUB
    // stack: gas_limit - intrinsic_gas
%endmacro
