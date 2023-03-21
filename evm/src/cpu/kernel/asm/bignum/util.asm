%macro memcpy_kernel_general
    // stack: dst, src, len
    %stack (dst, src, len) -> (0, @SEGMENT_KERNEL_GENERAL, dst, 0, @SEGMENT_KERNEL_GENERAL, src, len, %%after)
    %jump(memcpy)
%%after:
%endmacro

%macro clear_kernel_general
    // stack: dst, len
    %stack (dst, len) -> (0, @SEGMENT_KERNEL_GENERAL, dst, 0, len, %%after)
    %jump(memset)
%%after:
%endmacro
