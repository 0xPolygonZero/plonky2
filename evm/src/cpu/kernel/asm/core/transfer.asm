// Transfers some ETH from one address to another. The amount is given in wei.
// Pre stack: from, to, amount, retdest
// Post stack: (empty)
global transfer_eth:
    // stack: from, to, amount, retdest
    %stack (from, to, amount, retdest)
        -> (from, amount, to, amount)
    %deduct_eth
    // TODO: Handle exception from %deduct_eth?
    // stack: to, amount, retdest
    %add_eth
    // stack: retdest
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

global deduct_eth:
    // stack: addr, amount, retdest
    %jump(mpt_read_state_trie)
deduct_eth_after_read:
    PANIC // TODO

// Convenience macro to call deduct_eth and return where we left off.
%macro deduct_eth
    %stack (addr, amount) -> (addr, amount, %%after)
    %jump(deduct_eth)
%%after:
%endmacro

global add_eth:
    PANIC // TODO

// Convenience macro to call add_eth and return where we left off.
%macro add_eth
    %stack (addr, amount) -> (addr, amount, %%after)
    %jump(add_eth)
%%after:
%endmacro
