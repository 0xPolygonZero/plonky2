// Expects the stack to contain memory offset `ost_i` and current binary power `2^i`
%macro shift_table_store_2exp
    // stack: ost_i, 2^i
    DUP2
    DUP2
    // stack: ost_i, 2^i, ost_i, 2^i
    // stack: ost_i, 2^i, ost_i, 2^i
    %mstore_kernel(@SEGMENT_SHIFT_TABLE)
    // stack: ost_i, 2^i
    %add_const(32)  // TODO: Check that this is the right offset increment
    // stack: ost_(i+1), 2^i
    SWAP1
    // stack: 2^i, ost_(i+1)
    %mul_const(2)
    // stack: 2^(i+1), ost_(i+1)
    SWAP1
    // stack: ost_(i+1), 2^(i+1)
%endmacro

// Set segment[i] = 2^i for i = 0..255
%macro shift_table_init
    PUSH 1   // 2^0
    PUSH 0   // initial offset is zero
    // stack: ost, $1
    %rep 256  // TODO: Check that this doesn't alter the stack!
        // stack: ost_i, 2^i
        %shift_table_store_2exp
        // stack: ost_(i+1), 2^(i+1)
    %endrep
    %pop2
%endmacro
