%macro memcpy_current_general
    // stack: dst, src, len
    // DST and SRC are offsets, for the same memory segment
    GET_CONTEXT PUSH @SEGMENT_KERNEL_GENERAL %build_address_no_offset
    %stack (addr_no_offset, dst, src, len) -> (addr_no_offset, src, addr_no_offset, dst, len, %%after)
    ADD
    // stack: SRC, addr_no_offset, dst, len, %%after
    SWAP2
    ADD
    // stack: DST, SRC, len, %%after
    %jump(memcpy)
%%after:
%endmacro

%macro clear_current_general
    // stack: dst, len
    GET_CONTEXT PUSH @SEGMENT_KERNEL_GENERAL %build_address
    %stack (DST, len) -> (DST, len, %%after)
    %jump(memset)
%%after:
%endmacro
