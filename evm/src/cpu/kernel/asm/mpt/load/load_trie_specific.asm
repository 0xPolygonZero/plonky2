global mpt_load_state_trie_value:
    // stack: retdest

    // Load and append the nonce and balance.
    PROVER_INPUT(mpt) %append_to_trie_data
    PROVER_INPUT(mpt) %append_to_trie_data

    // Now increment the trie data size by 2, to leave room for our storage trie
    // pointer and code hash fields, before calling load_mpt which will append
    // our storage trie data.
    %get_trie_data_size
    // stack: storage_trie_ptr_ptr, retdest
    DUP1 %add_const(2)
    // stack: storage_trie_ptr, storage_trie_ptr_ptr, retdest
    %set_trie_data_size
    // stack: storage_trie_ptr_ptr, retdest

    %load_mpt(mpt_load_storage_trie_value)
    // stack: storage_trie_ptr, storage_trie_ptr_ptr, retdest
    DUP2 %mstore_trie_data
    // stack: storage_trie_ptr_ptr, retdest
    %increment
    // stack: code_hash_ptr, retdest
    PROVER_INPUT(mpt)
    // stack: code_hash, code_hash_ptr, retdest
    SWAP1 %mstore_trie_data
    // stack: retdest
    JUMP

global mpt_load_txn_trie_value:
    // stack: retdest
    PROVER_INPUT(mpt)
    // stack: rlp_len, retdest
    // The first element is the rlp length
    DUP1 %append_to_trie_data
    PUSH 0

mpt_load_loop:
    // stack: i, rlp_len, retdest
    DUP2 DUP2 EQ %jumpi(mpt_load_end)
    PROVER_INPUT(mpt) %append_to_trie_data
    %increment
    %jump(mpt_load_loop)

mpt_load_end:
    // stack: i, rlp_len, retdest
    %pop2
    JUMP

global mpt_load_receipt_trie_value:
    // stack: retdest
    // Load first byte. It is either `payload_len` or the transaction type.
    PROVER_INPUT(mpt) DUP1 %append_to_trie_data
    // If the first byte is less than 3, then it is the transaction type, equal to either 1 or 2. 
    // In that case, we still need to load the payload length.
    %lt_const(3) %jumpi(mpt_load_payload_len)
    
mpt_load_after_type:
    // Load status.
    PROVER_INPUT(mpt) %append_to_trie_data
    // Load cum_gas_used.
    PROVER_INPUT(mpt) %append_to_trie_data
    // Load bloom.
    %rep 256
        PROVER_INPUT(mpt) %append_to_trie_data
    %endrep
    // Load logs_payload_len.
    PROVER_INPUT(mpt) %append_to_trie_data
    // Load num_logs.
    PROVER_INPUT(mpt)
    DUP1
    %append_to_trie_data
    // stack: num_logs, retdest
    // Load logs.
    PUSH 0

mpt_load_receipt_trie_value_logs_loop:
    // stack: i, num_logs, retdest
    DUP2 DUP2 EQ
    // stack: i == num_logs, i, num_logs, retdest
    %jumpi(mpt_load_receipt_trie_value_end)
    // stack: i, num_logs, retdest
    // Load log_payload_len.
    PROVER_INPUT(mpt) %append_to_trie_data
    // Load address.
    PROVER_INPUT(mpt) %append_to_trie_data
    // Load num_topics.
    PROVER_INPUT(mpt)
    DUP1
    %append_to_trie_data
    // stack: num_topics, i, num_logs, retdest
    // Load topics.
    PUSH 0

mpt_load_receipt_trie_value_topics_loop:
    // stack: j, num_topics, i, num_logs, retdest
    DUP2 DUP2 EQ
    // stack: j == num_topics, j, num_topics, i, num_logs, retdest
    %jumpi(mpt_load_receipt_trie_value_topics_end)
    // stack: j, num_topics, i, num_logs, retdest
    // Load topic.
    PROVER_INPUT(mpt) %append_to_trie_data
    %increment
    %jump(mpt_load_receipt_trie_value_topics_loop)

mpt_load_receipt_trie_value_topics_end:
    // stack: num_topics, num_topics, i, num_logs, retdest
    %pop2
    // stack: i, num_logs, retdest
    // Load data_len.
    PROVER_INPUT(mpt) 
    DUP1
    %append_to_trie_data
    // stack: data_len, i, num_logs, retdest
    // Load data.
    PUSH 0

mpt_load_receipt_trie_value_data_loop:
    // stack: j, data_len, i, num_logs, retdest
    DUP2 DUP2 EQ
    // stack: j == data_len, j, data_len, i, num_logs, retdest
    %jumpi(mpt_load_receipt_trie_value_data_end)
    // stack: j, data_len, i, num_logs, retdest
    // Load data byte.
    PROVER_INPUT(mpt) %append_to_trie_data
    %increment
    %jump(mpt_load_receipt_trie_value_data_loop)

mpt_load_receipt_trie_value_data_end:
    // stack: data_len, data_len, i, num_logs, retdest
    %pop2
    %increment
    %jump(mpt_load_receipt_trie_value_logs_loop)

mpt_load_receipt_trie_value_end:
    // stack: num_logs, num_logs, retdest
    %pop2
    JUMP

mpt_load_payload_len:
    // stack: retdest
    PROVER_INPUT(mpt) %append_to_trie_data
    %jump(mpt_load_after_type)

global mpt_load_storage_trie_value:
    // stack: retdest
    PROVER_INPUT(mpt)
    %append_to_trie_data
    // stack: retdest
    JUMP
