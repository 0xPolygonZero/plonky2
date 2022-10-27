/// Initialise the lookup table of binary powers for doing left/right shifts
///
/// Specifically, set SHIFT_TABLE_SEGMENT[i] = 2^i for i = 0..255.
%macro shift_table_init:
    push 1   // 2^0
    push 0   // initial offset is zero
    // stack: ost_0, $1
    %rep 256
        // stack: ost_i, 2^i
        dup2
        dup2
        // stack: ost_i, 2^i, ost_i, 2^i
        %mstore_kernel(@SEGMENT_SHIFT_TABLE)
        // stack: ost_i, 2^i
        %increment  // FIXME: Check that this is the right offset increment
        // stack: ost_(i+1), 2^i
        swap1
        // stack: 2^i, ost_(i+1)
        %mul_const(2)
        // stack: 2^(i+1), ost_(i+1)
        swap1
        // stack: ost_(i+1), 2^(i+1)
    %endrep
    %pop2
