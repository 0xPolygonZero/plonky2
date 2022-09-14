// We put the message schedule in memory starting at 64 * num_blocks + 2.
%macro message_schedule_addr_from_num_blocks
    // stack: num_blocks
    %mul_const(64)
    %add_const(2)
%endmacro

// We use memory starting at 320 * num_blocks + 2 (after the message schedule
// space) as scratch space to store stack values.
%macro scratch_space_addr_from_num_blocks
    // stack: num_blocks
    %mul_const(320)
    %add_const(2)
%endmacro

%macro truncate_to_u32
    %and_const(0xFFFFFFFF)
%endmacro