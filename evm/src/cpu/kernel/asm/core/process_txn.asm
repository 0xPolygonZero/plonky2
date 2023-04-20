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

    %mload_txn_field(@TXN_FIELD_ORIGIN)
    // stack: sender, retdest

    // Check that txn nonce matches account nonce.
     DUP1 %nonce
    // stack: sender_nonce, sender, retdest
    %mload_txn_field(@TXN_FIELD_NONCE)
    // stack: tx_nonce, sender_nonce, sender, retdest
    %assert_eq
    // stack: sender, retdest

    // Assert sender has no code.
    DUP1 %ext_code_empty %assert_nonzero
    // stack: sender, retdest

    // Assert sender balance >= gas_limit * gas_price + value.
    %balance
    // stack: sender_balance, retdest
    %mload_txn_field(@TXN_FIELD_COMPUTED_FEE_PER_GAS)
    %mload_txn_field(@TXN_FIELD_GAS_LIMIT)
    MUL
    %mload_txn_field(@TXN_FIELD_VALUE)
    ADD
    %assert_le
    // stack: retdest

    // Assert chain ID matches block metadata
    %mload_txn_field(@TXN_FIELD_CHAIN_ID_PRESENT)
    // stack: chain_id_present, retdest
    DUP1
    %mload_txn_field(@TXN_FIELD_CHAIN_ID)
    // stack: tx_chain_id, chain_id_present, chain_id_present, retdest
    MUL SWAP1
    // stack: chain_id_present, filtered_tx_chain_id, retdest
    %mload_global_metadata(@GLOBAL_METADATA_BLOCK_CHAIN_ID)
    MUL
    // stack: filtered_block_chain_id, filtered_tx_chain_id, retdest
    %assert_eq
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

    // Create the new contract account in the state trie.
    DUP1
    %mload_txn_field(@TXN_FIELD_VALUE)
    // stack: value, address, address, retdest
    %create_contract_account
    // stack: status, address, retdest
    // It should be impossible to create address collisions with a contract creation txn,
    // since the address was derived from nonce, unlike with CREATE2.
    %jumpi(panic)
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
    %returndatasize // Size of the code.
    // stack: code_size, leftover_gas, new_ctx, address, retdest
    DUP1 %gt_const(@MAX_CODE_SIZE) %jumpi(panic) // TODO: need to revert changes here.
    // stack: code_size, leftover_gas, new_ctx, address, retdest
    %mul_const(@GAS_CODEDEPOSIT) SWAP1
    // stack: leftover_gas, codedeposit_cost, new_ctx, address, retdest
    DUP2 DUP2 LT %jumpi(panic) // TODO: need to revert changes here.
    // stack: leftover_gas, codedeposit_cost, new_ctx, address, retdest
    SUB
    // stack: leftover_gas, new_ctx, address, retdest
    %pay_coinbase_and_refund_sender
    // TODO: Delete accounts in self-destruct list and empty touched addresses.
    // stack: new_ctx, address, retdest
    POP
    POP
    JUMP

global process_message_txn:
    // stack: retdest
    %mload_txn_field(@TXN_FIELD_VALUE)
    %mload_txn_field(@TXN_FIELD_TO)
    DUP1 %insert_accessed_addresses_no_return
    %mload_txn_field(@TXN_FIELD_ORIGIN)
    DUP1 %insert_accessed_addresses_no_return
    // stack: from, to, amount, retdest
    %transfer_eth
    // stack: transfer_eth_status, retdest
    %jumpi(process_message_txn_insufficient_balance)
    // stack: retdest

    %handle_precompiles_from_eoa

    // If to's code is empty, return.
    %mload_txn_field(@TXN_FIELD_TO) %ext_code_empty
    // stack: code_empty, retdest
    %jumpi(process_message_txn_return)

    // Add precompiles to accessed addresses.
    PUSH @ECREC %insert_accessed_addresses_no_return
    PUSH @SHA256 %insert_accessed_addresses_no_return
    PUSH @RIP160 %insert_accessed_addresses_no_return
    PUSH @ID %insert_accessed_addresses_no_return
    PUSH @EXPMOD %insert_accessed_addresses_no_return
    PUSH @BN_ADD %insert_accessed_addresses_no_return
    PUSH @BN_MUL %insert_accessed_addresses_no_return
    PUSH @SNARKV %insert_accessed_addresses_no_return
    PUSH @BLAKE2_F %insert_accessed_addresses_no_return

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

    // Set calldatasize and copy txn data to calldata.
    %mload_txn_field(@TXN_FIELD_DATA_LEN)
    %stack (calldata_size, new_ctx, retdest) -> (calldata_size, new_ctx, calldata_size, retdest)
    %set_new_ctx_calldata_size
    %stack (new_ctx, calldata_size, retdest) -> (new_ctx, @SEGMENT_CALLDATA, 0, 0, @SEGMENT_TXN_DATA, 0, calldata_size, process_message_txn_code_loaded_finish, new_ctx, retdest)
    %jump(memcpy)

process_message_txn_code_loaded_finish:
    %enter_new_ctx
    // (Old context) stack: new_ctx, retdest

global process_message_txn_after_call:
    // stack: success, leftover_gas, new_ctx, retdest
    POP // TODO: Success will go into the receipt when we support that.
    // stack: leftover_gas, new_ctx, retdest
    %pay_coinbase_and_refund_sender
    // TODO: Delete accounts in self-destruct list and empty touched addresses.
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
