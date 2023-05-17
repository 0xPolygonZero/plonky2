// Create a smart contract account with the given address and the given endowment value.
// Pre stack: value, address
// Post stack: status
%macro create_contract_account
    // stack: value, address
    DUP2 %journal_add_account_created
    DUP2 %insert_touched_addresses
    DUP2 %mpt_read_state_trie
    // stack: existing_account_ptr, value, address
    // If the account doesn't exist, there's no need to check its balance or nonce,
    // so we can skip ahead, setting existing_balance = existing_account_ptr = 0.
    DUP1 ISZERO %jumpi(%%do_insert)

    // stack: existing_account_ptr, value, address
    DUP1 %mload_trie_data // nonce = account[0]
    // stack: nonce, existing_account_ptr, value, address
    %jumpi(%%error_nonzero_nonce)
    // stack: existing_account_ptr, value, address
    %increment %mload_trie_data // balance = account[1]

%%do_insert:
    // stack: existing_balance, value, address
    ADD
    // stack: new_acct_value, address
    // Write the new account's data to MPT data, and get a pointer to it.
    %get_trie_data_size
    // stack: account_ptr, new_acct_value, address
    PUSH 0 DUP4 %journal_add_nonce_change
    PUSH 1 %append_to_trie_data // nonce = 1
    // stack: account_ptr, new_acct_value, address
    SWAP1 %append_to_trie_data // balance = new_acct_value
    // stack: account_ptr, address
    PUSH 0 %append_to_trie_data // storage_root = nil
    // stack: account_ptr, address
    PUSH @EMPTY_STRING_HASH %append_to_trie_data // code_hash = keccak('')
    // stack: account_ptr, address
    SWAP1
    // stack: address, account_ptr
    %addr_to_state_key
    // stack: state_key, account_ptr
    %mpt_insert_state_trie
    // stack: (empty)
    PUSH 0 // success
    %jump(%%end)

// If the nonce is nonzero, that means a contract has already been deployed to this address.
// (This should be impossible with contract creation transactions or CREATE, but possible with CREATE2.)
// So we return 1 to indicate an error.
%%error_nonzero_nonce:
    %stack (existing_account_ptr, address, value) -> (1)

%%end:
    // stack: status
%endmacro
