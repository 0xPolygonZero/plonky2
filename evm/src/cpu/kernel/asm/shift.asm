/// Initialise the lookup table of binary powers for doing left/right shifts
///
/// Specifically, set SHIFT_TABLE_SEGMENT[i] = 2^i for i = 0..255.
%macro shift_table_init
    push @SEGMENT_SHIFT_TABLE  // segment, ctx == virt == 0
    push 1                     // 2^0
    %rep 255
        // stack: 2^i, addr_i
        dup2
        %increment
        // stack: addr_(i+1), 2^i, addr_i
        dup2
        dup1
        add
        // stack: 2^(i+1), addr_(i+1), 2^i, addr_i
    %endrep
    %rep 256
        mstore_general
    %endrep
%endmacro
