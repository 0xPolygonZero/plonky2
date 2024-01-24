%macro memcpy_current_general
    // stack: dst, src, len
    // DST and SRC are offsets, for the same memory segment
    %build_current_general_address_no_offset
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
    %build_current_general_address
    %stack (DST, len) -> (DST, len, %%after)
    %jump(memset)
%%after:
%endmacro
