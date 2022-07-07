global process_txn:
    JUMPDEST
    // stack: (empty)
    NEW_CONTEXT
    // stack: (empty)

    // We will peak at the first byte to determine what type of transaction this is.
    // Note that type 1 and 2 transactions have a first byte of 1 and 2, respectively.
    // This is outside of the transaction's RLP data, so we strip it off here.
    // Type 0 (legacy) transactions have no such prefix, but their RLP will have a
    // first byte >= 0xc0, so there is no overlap.

    PUSH 0xc0
    // stack: 0xc0
    PEAK_INPUT // Don't consume the input byte yet, as it may be part of the RLP (if type 0).
    // stack: first_byte, 0xc0
    GE
    // stack: first_byte >= 0xc0
    PUSH process_type_0_txn
    JUMPI

    // stack: (empty)
    INPUT
    // stack: first_byte
    PUSH 1
    EQ
    // stack: first_byte == 1
    PUSH process_type_1_txn
    JUMPI

    // stack: (empty)
    // At this point, since it's not a type 0 or 1 transaction, it must be a type 2 transaction.
    PUSH process_type_2_txn
    JUMP
