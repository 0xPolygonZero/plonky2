// After the transaction data has been parsed into a normalized set of fields
// (see NormalizedTxnField), this routine processes the transaction.

// TODO: Save checkpoints in @CTX_METADATA_STATE_TRIE_CHECKPOINT_PTR and @SEGMENT_STORAGE_TRIE_CHECKPOINT_PTRS.

// Pre stack: retdest
// Post stack: success, leftover_gas
global process_normalized_txn:
    // stack: retdest
    %compute_fees
    // stack: retdest

    // Compute this transaction's intrinsic gas and store it.
    %intrinsic_gas
    DUP1
    %mstore_txn_field(@TXN_FIELD_INTRINSIC_GAS)
    // stack: intrinsic_gas, retdest

    // Assert gas_limit >= intrinsic_gas.
    %mload_txn_field(@TXN_FIELD_GAS_LIMIT)
    %assert_ge(invalid_txn)

    // Assert block gas limit >= txn gas limit.
    %mload_txn_field(@TXN_FIELD_GAS_LIMIT)
    %mload_global_metadata(@GLOBAL_METADATA_BLOCK_GAS_LIMIT)
    %assert_ge(invalid_txn)

    %mload_txn_field(@TXN_FIELD_ORIGIN)
    // stack: sender, retdest

    // Check that txn nonce matches account nonce.
    DUP1 %nonce
    DUP1 %eq_const(@MAX_NONCE) %assert_zero(invalid_txn_2) // EIP-2681
    // stack: sender_nonce, sender, retdest
    %mload_txn_field(@TXN_FIELD_NONCE)
    // stack: tx_nonce, sender_nonce, sender, retdest
    %assert_eq(invalid_txn_1)
    // stack: sender, retdest

    // Assert sender has no code.
    DUP1 %ext_code_empty %assert_nonzero(invalid_txn_1)
    // stack: sender, retdest

    // Assert sender balance >= gas_limit * gas_price + value.
    %balance
    // stack: sender_balance, retdest
    %mload_txn_field(@TXN_FIELD_COMPUTED_FEE_PER_GAS)
    %mload_txn_field(@TXN_FIELD_GAS_LIMIT)
    MUL
    %mload_txn_field(@TXN_FIELD_VALUE)
    ADD
    %assert_le(invalid_txn)
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
    %assert_eq(invalid_txn)
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
    DUP1 %increment_nonce

global warm_origin:
    // stack: origin, retdest
    %insert_accessed_addresses_no_return

global warm_precompiles:
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

// EIP-3651
global warm_coinbase:
    %mload_global_metadata(@GLOBAL_METADATA_BLOCK_BENEFICIARY)
    %insert_accessed_addresses_no_return

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
    %decrement // Need the non-incremented nonce
    SWAP1
    // stack: origin, origin_nonce, retdest
    %get_create_address
    // stack: address, retdest
    DUP1 %insert_accessed_addresses_no_return

    %checkpoint

    // Create the new contract account in the state trie.
    DUP1
    // stack: address, address, retdest
    %create_contract_account
    // stack: status, address, retdest
    %jumpi(create_contract_account_fault)

    // stack: address, retdest
    // Transfer value to new contract
    DUP1 %mload_txn_field(@TXN_FIELD_VALUE)
    SWAP1
    %mload_txn_field(@TXN_FIELD_ORIGIN)
    DUP3 DUP3 DUP3
    %transfer_eth %jumpi(panic)
    %journal_add_balance_transfer
    // stack: address, retdest

    %create_context
    // stack: new_ctx, address, retdest

    // Store constructor code length
    PUSH @CTX_METADATA_CODE_SIZE
    // stack: offset, new_ctx, address, retdest
    DUP2 // new_ctx
    ADD // CTX_METADATA_CODE_SIZE is already scaled by its segment
    // stack: addr, new_ctx, address, retdest
    %mload_txn_field(@TXN_FIELD_DATA_LEN)
    // stack: data_len, addr, new_ctx, address, retdest
    MSTORE_GENERAL
    // stack: new_ctx, address, retdest

    // Copy the code from txdata to the new context's code segment.
    PUSH process_contract_creation_txn_after_code_loaded
    %mload_txn_field(@TXN_FIELD_DATA_LEN)
    PUSH @SEGMENT_TXN_DATA // SRC (context == offset == 0)
    DUP4 // DST (segment == 0 (i.e. CODE), and offset == 0)
    %jump(memcpy_bytes)

global process_contract_creation_txn_after_code_loaded:
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
    // We eventually return leftover_gas and success.
    %stack (success, leftover_gas, new_ctx, address, retdest) -> (success, leftover_gas, new_ctx, address, retdest, success)

    ISZERO %jumpi(contract_creation_fault_3)

    // EIP-3541: Reject new contract code starting with the 0xEF byte
    PUSH 0 %mload_current(@SEGMENT_RETURNDATA) %eq_const(0xEF) %jumpi(contract_creation_fault_3_zero_leftover)

    // stack: leftover_gas, new_ctx, address, retdest, success
    %returndatasize // Size of the code.
    // stack: code_size, leftover_gas, new_ctx, address, retdest, success
    DUP1 %gt_const(@MAX_CODE_SIZE) %jumpi(contract_creation_fault_4)
    // stack: code_size, leftover_gas, new_ctx, address, retdest, success
    %mul_const(@GAS_CODEDEPOSIT) SWAP1
    // stack: leftover_gas, codedeposit_cost, new_ctx, address, retdest, success
    DUP2 DUP2 LT %jumpi(contract_creation_fault_4)
    // stack: leftover_gas, codedeposit_cost, new_ctx, address, retdest, success
    SUB

    // Store the code hash of the new contract.
    // stack: leftover_gas, new_ctx, address, retdest, success
    %returndatasize
    PUSH @SEGMENT_RETURNDATA
    GET_CONTEXT
    %build_address_no_offset
    // stack: addr, len
    KECCAK_GENERAL
    // stack: codehash, leftover_gas, new_ctx, address, retdest, success
    %observe_new_contract
    DUP4
    // stack: address, codehash, leftover_gas, new_ctx, address, retdest, success
    %set_codehash

    %stack (leftover_gas, new_ctx, address, retdest, success) -> (leftover_gas, new_ctx, address, retdest, success, leftover_gas)
    %pay_coinbase_and_refund_sender
    // stack: leftover_gas', new_ctx, address, retdest, success, leftover_gas
    SWAP5 POP
    %delete_all_touched_addresses
    %delete_all_selfdestructed_addresses
    // stack: new_ctx, address, retdest, success, leftover_gas
    POP
    POP
    JUMP

global process_message_txn:
    // stack: retdest
    %mload_txn_field(@TXN_FIELD_VALUE)
    %mload_txn_field(@TXN_FIELD_TO)
    DUP1 %insert_accessed_addresses_no_return
    %mload_txn_field(@TXN_FIELD_ORIGIN)
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

    // Otherwise, load to's code and execute it in a new context.
    // stack: retdest
    %create_context
    // stack: new_ctx, retdest
    PUSH process_message_txn_code_loaded
    DUP2 // new_ctx
    %mload_txn_field(@TXN_FIELD_TO)
    // stack: address, new_ctx, process_message_txn_code_loaded, new_ctx, retdest
    %jump(load_code_padded)

global process_message_txn_insufficient_balance:
    // stack: retdest
    PANIC // TODO

global process_message_txn_return:
    // stack: retdest
    // Since no code was executed, the leftover gas is the non-intrinsic gas.
    %non_intrinisic_gas
    DUP1
    // stack: leftover_gas, leftover_gas, retdest
    %pay_coinbase_and_refund_sender
    // stack: leftover_gas', leftover_gas, retdest
    SWAP1 POP
    %delete_all_touched_addresses
    // stack: leftover_gas', retdest
    SWAP1
    PUSH 1 // success
    SWAP1
    // stack: retdest, success, leftover_gas
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
    %stack (new_ctx, calldata_size, retdest) -> (new_ctx, @SEGMENT_CALLDATA, @SEGMENT_TXN_DATA, calldata_size, process_message_txn_code_loaded_finish, new_ctx, retdest)
    %build_address_no_offset // DST
    %jump(memcpy_bytes)

process_message_txn_code_loaded_finish:
    %enter_new_ctx
    // (Old context) stack: new_ctx, retdest

global process_message_txn_after_call:
    // stack: success, leftover_gas, new_ctx, retdest
    // We will return leftover_gas and success.
    %stack (success, leftover_gas, new_ctx, retdest) -> (success, leftover_gas, new_ctx, retdest, success, leftover_gas)
    ISZERO %jumpi(process_message_txn_fail)
process_message_txn_after_call_contd:
    // stack: leftover_gas, new_ctx, retdest, success, leftover_gas
    %pay_coinbase_and_refund_sender
    // stack: leftover_gas', new_ctx, retdest, success, leftover_gas
    SWAP4 POP
    %delete_all_touched_addresses
    %delete_all_selfdestructed_addresses
    // stack: new_ctx, retdest, success, leftover_gas
    POP
    JUMP

process_message_txn_fail:
    // stack: leftover_gas, new_ctx, retdest, success, leftover_gas
    // Transfer value back to the caller.
    %mload_txn_field(@TXN_FIELD_VALUE) ISZERO %jumpi(process_message_txn_after_call_contd)
    %mload_txn_field(@TXN_FIELD_VALUE)
    %mload_txn_field(@TXN_FIELD_ORIGIN)
    %mload_txn_field(@TXN_FIELD_TO)
    %transfer_eth %jumpi(panic)
    %jump(process_message_txn_after_call_contd)

%macro pay_coinbase_and_refund_sender
    // stack: leftover_gas
    DUP1
    // stack: leftover_gas, leftover_gas
    %mload_txn_field(@TXN_FIELD_GAS_LIMIT)
    SUB
    // stack: used_gas, leftover_gas
    %mload_global_metadata(@GLOBAL_METADATA_REFUND_COUNTER)
    // stack: refund, used_gas, leftover_gas
    DUP2 %div_const(@MAX_REFUND_QUOTIENT) // max_refund = used_gas/5
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
    DUP1

    // Refund gas to the origin.
    %mload_txn_field(@TXN_FIELD_COMPUTED_FEE_PER_GAS)
    MUL
    // stack: leftover_gas_cost, leftover_gas'
    %mload_txn_field(@TXN_FIELD_ORIGIN)
    // stack: origin, leftover_gas_cost, leftover_gas'
    %add_eth
    // stack: leftover_gas'
%endmacro

// Sets @TXN_FIELD_MAX_FEE_PER_GAS and @TXN_FIELD_MAX_PRIORITY_FEE_PER_GAS.
%macro compute_fees
    // stack: (empty)
    %mload_global_metadata(@GLOBAL_METADATA_BLOCK_BASE_FEE)
    %mload_txn_field(@TXN_FIELD_MAX_PRIORITY_FEE_PER_GAS)
    %mload_txn_field(@TXN_FIELD_MAX_FEE_PER_GAS)
    // stack: max_fee, max_priority_fee, base_fee
    DUP3 DUP2 %assert_ge(invalid_txn_3) // Assert max_fee >= base_fee
    // stack: max_fee, max_priority_fee, base_fee
    DUP2 DUP2 %assert_ge(invalid_txn_3) // Assert max_fee >= max_priority_fee
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

create_contract_account_fault:
    %revert_checkpoint
    // stack: address, retdest
    POP
    PUSH 0 // leftover_gas
    // stack: leftover_gas, retdest
    %pay_coinbase_and_refund_sender
    // stack: leftover_gas', retdest
    %delete_all_touched_addresses
    %delete_all_selfdestructed_addresses
    // stack: leftover_gas', retdest
    SWAP1 PUSH 0 // success
    // stack: success, retdest, leftover_gas
    SWAP1
    JUMP

contract_creation_fault_3:
    %revert_checkpoint
    %stack (leftover_gas, new_ctx, address, retdest, success) -> (leftover_gas, retdest, success)
    %pay_coinbase_and_refund_sender
    // stack: leftover_gas', retdest, success
    %delete_all_touched_addresses
    %delete_all_selfdestructed_addresses
    %stack (leftover_gas, retdest, success) -> (retdest, 0, leftover_gas)
    JUMP

contract_creation_fault_3_zero_leftover:
    %revert_checkpoint
    // stack: leftover_gas, new_ctx, address, retdest, success
    %pop3
    PUSH 0 // leftover gas
    // stack: leftover_gas, retdest, success
    %pay_coinbase_and_refund_sender
    %delete_all_touched_addresses
    %delete_all_selfdestructed_addresses
    %stack (leftover_gas, retdest, success) -> (retdest, 0, leftover_gas)
    JUMP

contract_creation_fault_4:
    %revert_checkpoint
    // stack: code_size/leftover_gas, leftover_gas/codedeposit_cost, new_ctx, address, retdest, success
    %pop4
    PUSH 0 // leftover gas
    // stack: leftover_gas, retdest, success
    %pay_coinbase_and_refund_sender
    %delete_all_touched_addresses
    %delete_all_selfdestructed_addresses
    %stack (leftover_gas, retdest, success) -> (retdest, 0, leftover_gas)
    JUMP


global invalid_txn:
    POP
    %mload_txn_field(@TXN_FIELD_GAS_LIMIT)
    PUSH 0
    %jump(txn_after)

global invalid_txn_1:
    %pop2
    %mload_txn_field(@TXN_FIELD_GAS_LIMIT)
    PUSH 0
    %jump(txn_after)

global invalid_txn_2:
    %pop3
    %mload_txn_field(@TXN_FIELD_GAS_LIMIT)
    PUSH 0
    %jump(txn_after)

global invalid_txn_3:
    %pop4
    %mload_txn_field(@TXN_FIELD_GAS_LIMIT)
    PUSH 0
    %jump(txn_after)
