%macro message_schedule_addr_from_num_blocks
    // stack: num_blocks
    %mul_const(64)
    %add_const(2)
%endmacro

%macro scratch_space_addr_from_num_blocks
    // stack: num_blocks
    %mul_const(320)
    %add_const(2)
%endmacro