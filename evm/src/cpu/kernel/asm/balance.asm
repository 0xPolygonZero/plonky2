%macro balance
    // stack: address
    %mpt_read_state_trie
    // stack: account_ptr
    %add_const(1)
    // stack: balance_ptr
    %mload_trie_data
    // stack: balance
%endmacro

global balance:
    // stack: address, retdest
    %balance
    // stack: balance, retdest
    SWAP1 JUMP

%macro selfbalance
    // stack: (empty)
    ADDRESS
    %balance
%endmacro
