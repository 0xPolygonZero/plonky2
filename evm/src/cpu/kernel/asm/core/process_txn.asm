// After the transaction data has been parsed into a normalized set of fields
// (see NormalizedTxnField), this routine processes the transaction.

// TODO: Save checkpoints in @CTX_METADATA_STATE_TRIE_CHECKPOINT_PTR and @SEGMENT_STORAGE_TRIE_CHECKPOINT_PTRS.

global process_normalized_txn:
    // stack: (empty)
    PUSH validate
    %jump(intrinsic_gas)

validate:
    // stack: intrinsic_gas
    // TODO: Check gas >= intrinsic_gas.
    // TODO: Check sender_balance >= intrinsic_gas + value.

buy_gas:
    // TODO: Deduct gas from sender (some may be refunded later).

increment_nonce:
    // TODO: Increment nonce.

process_based_on_type:
    %is_contract_creation
    %jumpi(process_contract_creation_txn)
    %jump(process_message_txn)

process_contract_creation_txn:
    // stack: (empty)
    // Push the code address & length onto the stack, then call `create`.
    %mload_txn_field(@TXN_FIELD_DATA_LEN)
    // stack: code_len
    PUSH 0
    // stack: code_offset, code_len
    PUSH @SEGMENT_TXN_DATA
    // stack: code_segment, code_offset, code_len
    PUSH 0 // context
    // stack: CODE_ADDR, code_len
    %jump(create)

process_message_txn:
    // TODO
