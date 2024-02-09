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

// Returns 0 on success, or 1 if addr has insufficient balance. Panics if addr isn't found in the trie.
// Pre stack: addr, amount, retdest
// Post stack: status (0 indicates success)
global deduct_eth:
    // stack: addr, amount, retdest
    DUP1 %insert_touched_addresses
    DUP2 ISZERO %jumpi(deduct_eth_noop)
    DUP1 %key_balance %smt_read_state
    // stack: balance_ptr, addr, amount, retdest
    DUP1 %mload_trie_data
    // stack: balance, balance_ptr, addr, amount, retdest
    DUP1 DUP5 GT
    // stack: amount > balance, balance, balance_ptr, addr, amount, retdest
    %jumpi(deduct_eth_insufficient_balance)
    // stack: balance, balance_ptr, addr, amount, retdest
    DUP1 DUP5 EQ
    // stack: amount == balance, balance, balance_ptr, addr, amount, retdest
    %jumpi(deduct_eth_delete_balance)
    %stack (balance, balance_ptr, addr, amount, retdest) -> (balance, amount, balance_ptr, retdest, 0)
    SUB
    SWAP1
    // stack: balance_ptr, balance - amount, retdest, 0
    %mstore_trie_data
    // stack: retdest, 0
    JUMP
deduct_eth_insufficient_balance:
    %stack (balance, balance_ptr, addr, amount, retdest) -> (retdest, 1)
    JUMP
deduct_eth_delete_balance:
    %stack (balance, balance_ptr, addr, amount, retdest) -> (addr, retdest, 1)
    %key_balance %smt_delete_state
    // stack: retdest, 1
    JUMP
deduct_eth_noop:
    %stack (addr, amount, retdest) -> (retdest, 0)
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
    DUP1 %insert_touched_addresses
    DUP2 ISZERO %jumpi(add_eth_noop)
    // stack: addr, amount, retdest
    DUP1 %key_code %smt_read_state %mload_trie_data
    // stack: codehash, addr, amount, retdest
    ISZERO %jumpi(add_eth_new_account) // If the account is empty, we need to create the account.
    // stack: addr, amount, retdest
    %key_balance DUP1 %smt_read_state
    DUP1 ISZERO %jumpi(add_eth_zero_balance)
    %stack (balance_ptr, key_balance, amount) -> (balance_ptr, amount, balance_ptr)
    // stack: balance_ptr, amount, balance_ptr, retdest
    %mload_trie_data ADD
    // stack: balance+amount, balance_ptr, retdest
    SWAP1 %mstore_trie_data
    JUMP
add_eth_zero_balance:
    // stack: balance_ptr, key_balance, amount, retdest
    POP
    // stack: key_balance, amount, retdest
    %smt_insert_state
    // stack: retdest
    JUMP

global add_eth_new_account:
    // stack: addr, amount, retdest
    DUP1 %journal_add_account_created
    // stack: addr, amount, retdest
    DUP1 %key_code
    %stack (key_code) -> (key_code, @EMPTY_STRING_POSEIDON_HASH)
    %smt_insert_state
    // stack: addr, amount, retdest
    %key_balance
    // stack: key_balance, amount, retdest
    %smt_insert_state
    JUMP

add_eth_noop:
    // stack: addr, amount, retdest
    %pop2 JUMP

// Convenience macro to call add_eth and return where we left off.
%macro add_eth
    %stack (addr, amount) -> (addr, amount, %%after)
    %jump(add_eth)
%%after:
%endmacro
