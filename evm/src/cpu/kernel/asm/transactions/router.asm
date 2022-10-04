// This is the entry point of transaction processing. We load the transaction
// RLP data into memory, check the transaction type, then based on the type we
// jump to the appropriate transaction parsing method.

global route_txn:
    // stack: retdest
    // First load transaction data into memory, where it will be parsed.
    PUSH read_txn_from_memory
    %jump(read_rlp_to_memory)

// At this point, the raw txn data is in memory.
read_txn_from_memory:
    // stack: retdest

    // We will peak at the first byte to determine what type of transaction this is.
    // Note that type 1 and 2 transactions have a first byte of 1 and 2, respectively.
    // Type 0 (legacy) transactions have no such prefix, but their RLP will have a
    // first byte >= 0xc0, so there is no overlap.

    PUSH 0
    %mload_current(@SEGMENT_RLP_RAW)
    %eq_const(1)
    // stack: first_byte == 1, retdest
    %jumpi(process_type_1_txn)
    // stack: retdest

    PUSH 0
    %mload_current(@SEGMENT_RLP_RAW)
    %eq_const(2)
    // stack: first_byte == 2, retdest
    %jumpi(process_type_2_txn)
    // stack: retdest

    // At this point, since it's not a type 1 or 2 transaction,
    // it must be a legacy (aka type 0) transaction.
    %jump(process_type_0_txn)
