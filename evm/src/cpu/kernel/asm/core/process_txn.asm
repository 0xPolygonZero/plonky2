// After the transaction data has been parsed into a normalized set of fields
// (see NormalizedTxnField), this routine processes the transaction.

// TODO: Save checkpoints in @CTX_METADATA_STATE_TRIE_CHECKPOINT_PTR and @SEGMENT_STORAGE_TRIE_CHECKPOINT_PTRS.

// Pre stack: retdest
// Post stack: (empty)
global process_normalized_txn:
    // stack: retdest
    PUSH validate
    %jump(intrinsic_gas)

global validate:
    // stack: intrinsic_gas, retdest
    // TODO: Check signature? (Or might happen in type_0.asm etc.)
    // TODO: Assert nonce is correct.
    // TODO: Assert sender has no code.
    POP // TODO: Assert gas_limit >= intrinsic_gas.
    // stack: retdest

global charge_gas:
    // TODO: Deduct gas limit from sender (some gas may be refunded later).

    PUSH 0 // TODO: Push sender.
    %increment_nonce

global process_based_on_type:
    %is_contract_creation
    %jumpi(process_contract_creation_txn)
    %jump(process_message_txn)

global process_contract_creation_txn:
    // stack: retdest
    // Push the code address & length onto the stack, then call `create`.
    %mload_txn_field(@TXN_FIELD_DATA_LEN)
    // stack: code_len, retdest
    PUSH 0
    // stack: code_offset, code_len, retdest
    PUSH @SEGMENT_TXN_DATA
    // stack: code_segment, code_offset, code_len, retdest
    PUSH 0 // context
    // stack: CODE_ADDR, code_len, retdest
    %jump(create)

global process_message_txn:
    // stack: retdest
    %mload_txn_field(@TXN_FIELD_VALUE)
    %mload_txn_field(@TXN_FIELD_TO)
    %mload_txn_field(@TXN_FIELD_ORIGIN)
    // stack: from, to, amount, retdest
    %transfer_eth
    // stack: transfer_eth_status, retdest
    %jumpi(process_message_txn_insufficient_balance)
    // stack: retdest
    // TODO: If code is non-empty, execute it in a new context.
    JUMP

global process_message_txn_insufficient_balance:
    // stack: retdest
    PANIC // TODO
