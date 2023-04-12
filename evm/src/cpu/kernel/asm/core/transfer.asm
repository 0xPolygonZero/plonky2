// Transfers some ETH from one address to another. The amount is given in wei.
// Pre stack: from, to, amount, retdest
// Post stack: status (0 indicates success)
global transfer_eth:
    // stack: from, to, amount, retdest
    %stack (from, to, amount, retdest)
        -> (from, amount, to, amount, retdest)
    %deduct_eth
    // stack: deduct_eth_status, to, amount, retdest
    %jumpi(transfer_eth_failure)
    // stack: to, amount, retdest
    %add_eth
    %stack (retdest) -> (retdest, 0)
    JUMP
global transfer_eth_failure:
    %stack (to, amount, retdest) -> (retdest, 1)
    JUMP

// Convenience macro to call transfer_eth and return where we left off.
%macro transfer_eth
    %stack (from, to, amount) -> (from, to, amount, %%after)
    %jump(transfer_eth)
%%after:
%endmacro

// Pre stack: should_transfer, from, to, amount
// Post stack: (empty)
%macro maybe_transfer_eth
    %jumpi(%%transfer)
    // We're skipping the transfer, so just pop the arguments and return.
    %pop3
    %jump(%%after)
%%transfer:
    %transfer_eth
%%after:
%endmacro

// Returns 0 on success, or 1 if addr has insufficient balance. Panics if addr isn't found in the trie.
// Pre stack: addr, amount, retdest
// Post stack: status (0 indicates success)
global deduct_eth:
    // stack: addr, amount, retdest
    %mpt_read_state_trie
    // stack: account_ptr, amount, retdest
    DUP1 ISZERO %jumpi(deduct_eth_no_such_account) // If the account pointer is null, return 1.
    %add_const(1)
    // stack: balance_ptr, amount, retdest
    DUP1 %mload_trie_data
    // stack: balance, balance_ptr, amount, retdest
    DUP1 DUP4 GT
    // stack: amount > balance, balance, balance_ptr, amount, retdest
    %jumpi(deduct_eth_insufficient_balance)
    %stack (balance, balance_ptr, amount, retdest) -> (balance, amount, balance_ptr, retdest, 0)
    SUB
    SWAP1
    // stack: balance_ptr, balance - amount, retdest, 0
    %mstore_trie_data
    // stack: retdest, 0
    JUMP
global deduct_eth_no_such_account:
    %stack (account_ptr, amount, retdest) -> (retdest, 1)
    JUMP
global deduct_eth_insufficient_balance:
    %stack (balance, balance_ptr, amount, retdest) -> (retdest, 1)
    JUMP

// Convenience macro to call deduct_eth and return where we left off.
%macro deduct_eth
    %stack (addr, amount) -> (addr, amount, %%after)
    %jump(deduct_eth)
%%after:
%endmacro

// Pre stack: addr, amount, redest
// Post stack: (empty)
global add_eth:
    // stack: addr, amount, retdest
    DUP1 %mpt_read_state_trie
    // stack: account_ptr, addr, amount, retdest
    DUP1 ISZERO %jumpi(add_eth_new_account) // If the account pointer is null, we need to create the account.
    %add_const(1)
    // stack: balance_ptr, addr, amount, retdest
    DUP1 %mload_trie_data
    // stack: balance, balance_ptr, addr, amount, retdest
    %stack (balance, balance_ptr, addr, amount) -> (amount, balance, balance_ptr)
    ADD
    // stack: new_balance, balance_ptr, retdest
    SWAP1
    // stack: balance_ptr, new_balance, retdest
    %mstore_trie_data
    // stack: retdest
    JUMP
global add_eth_new_account:
    // TODO: Skip creation if amount == 0?
    // stack: null_account_ptr, addr, amount, retdest
    POP
    %get_trie_data_size // pointer to new account we're about to create
    // stack: new_account_ptr, addr, amount, retdest
    SWAP2
    // stack: amount, addr, new_account_ptr, retdest
    PUSH 0 %append_to_trie_data // nonce
    %append_to_trie_data // balance
    // stack: addr, new_account_ptr, retdest
    PUSH 0 %append_to_trie_data // storage root pointer
    PUSH @EMPTY_STRING_HASH %append_to_trie_data // code hash
    // stack: addr, new_account_ptr, retdest
    %addr_to_state_key
    // stack: key, new_account_ptr, retdest
    %jump(mpt_insert_state_trie)

// Convenience macro to call add_eth and return where we left off.
%macro add_eth
    %stack (addr, amount) -> (addr, amount, %%after)
    %jump(add_eth)
%%after:
%endmacro
