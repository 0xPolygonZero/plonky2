// This is the entry point of transaction processing. We load the transaction
// RLP data into memory, check the transaction type, then based on the type we
// jump to the appropriate transaction parsing method.

global route_txn:
    // stack: txn_counter, num_nibbles, retdest
    // First load transaction data into memory, where it will be parsed.
    %stack(txn_counter, num_nibbles) -> (update_txn_trie, txn_counter, num_nibbles, read_txn_from_memory)
    // stack: update_txn_trie, txn_counter, num_nibbles, read_txn_from_memory, retdest
    %jump(read_rlp_to_memory)

// At this point, the raw txn data is in memory.
read_txn_from_memory:
    // stack: retdest

    // We will peak at the first byte to determine what type of transaction this is.
    // Note that type 1 and 2 transactions have a first byte of 1 and 2, respectively.
    // Type 0 (legacy) transactions have no such prefix, but their RLP will have a
    // first byte >= 0xc0, so there is no overlap.

    PUSH @SEGMENT_RLP_RAW // ctx == virt == 0
    MLOAD_GENERAL
    %eq_const(1)
    // stack: first_byte == 1, retdest
    %jumpi(process_type_1_txn)
    // stack: retdest

    PUSH @SEGMENT_RLP_RAW // ctx == virt == 0
    MLOAD_GENERAL
    %eq_const(2)
    // stack: first_byte == 2, retdest
    %jumpi(process_type_2_txn)
    // stack: retdest

    // At this point, since it's not a type 1 or 2 transaction,
    // it must be a legacy (aka type 0) transaction.
    %jump(process_type_0_txn)

global update_txn_trie:
    // stack: txn_rlp_len, txn_counter, num_nibbles, retdest
    // Copy the transaction rlp to the trie data segment.
    %get_trie_data_size
    // stack: value_ptr, txn_rlp_len, txn_counter, num_nibbles, retdest
    SWAP1
    // First we write txn rlp length
    DUP1 %append_to_trie_data
    // stack: txn_rlp_len, value_ptr, txn_counter, num_nibbles, ret_dest
    DUP2 %increment
    // stack: rlp_start=value_ptr+1, txn_rlp_len, value_ptr, txn_counter, num_nibbles, retdest
    

    // and now copy txn_rlp to the new block
    %stack (rlp_start, txn_rlp_len, value_ptr, txn_counter, num_nibbles) -> (
        @SEGMENT_RLP_RAW, // src addr. ctx == virt == 0
        rlp_start, @SEGMENT_TRIE_DATA, // swapped dest addr, ctx == 0
        txn_rlp_len, // mcpy len
        txn_rlp_len, rlp_start, txn_counter, num_nibbles, value_ptr)
    SWAP2 %build_kernel_address
    // stack: DST, SRC, txn_rlp_len, txn_rlp_len, rlp_start, txn_counter, num_nibbles, value_ptr
    %memcpy_bytes
    ADD
    %set_trie_data_size
    // stack: txn_counter, num_nibbles, value_ptr, retdest
    %jump(mpt_insert_txn_trie)
