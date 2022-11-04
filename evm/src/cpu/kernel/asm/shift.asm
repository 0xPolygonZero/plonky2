/// Initialise the lookup table of binary powers for doing left/right shifts
///
/// Specifically, set SHIFT_TABLE_SEGMENT[i] = 2^i for i = 0..255.
%macro shift_table_init
    push 1                     // 2^0
    push 0                     // initial offset is zero
    push @SEGMENT_SHIFT_TABLE  // segment
    dup2                       // kernel context is 0
    %rep 255
        // stack: context, segment, ost_i, 2^i
        dup4
        dup1
        add
        // stack: 2^(i+1), context, segment, ost_i, 2^i
        dup4
        %increment
        // stack: ost_(i+1), 2^(i+1), context, segment, ost_i, 2^i
        dup4
        dup4
        // stack: context, segment, ost_(i+1), 2^(i+1), context, segment, ost_i, 2^i
    %endrep
    %rep 256
        mstore_general
    %endrep
%endmacro
