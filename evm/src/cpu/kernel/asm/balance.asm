global balance:
    // stack: address, retdest
    %mpt_read_state_trie
    // stack: account_ptr, retdest
    DUP1 ISZERO %jumpi(retzero) // If the account pointer is null, return 0.
    %add_const(1)
    // stack: balance_ptr
    %mload_trie_data
    // stack: balance, retdest
    SWAP1 JUMP

retzero:
    %stack (account_ptr, retdest) -> (retdest, 0)
    JUMP


global selfbalance:
    // stack: retdest
    %address
    PUSH balance
    // stack: balance, address, retdest
    JUMP

