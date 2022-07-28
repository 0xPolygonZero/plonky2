// Transfers some ETH from one address to another. The amount is given in wei.
// Pre stack: from, to, amount, retdest
// Post stack: (empty)

global transfer_eth:
    // stack: from, to, amount, retdest
    // TODO: Replace with actual implementation.
    %pop3
    JUMP

// Convenience macro to call transfer_eth and return where we left off.
%macro transfer_eth
    %stack (from, to, amount) -> (from, to, amount, %%after)
    %jump(transfer_eth)
%%after:
%endmacro
