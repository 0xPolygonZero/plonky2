%macro memcpy_current_general
    // stack: dst, src, len
    GET_CONTEXT
    %stack (context, dst, src, len) -> (context, @SEGMENT_KERNEL_GENERAL, dst, context, @SEGMENT_KERNEL_GENERAL, src, len, %%after)
    %jump(memcpy)
%%after:
%endmacro

%macro clear_current_general
    // stack: dst, len
    GET_CONTEXT
    %stack (context, dst, len) -> (context, @SEGMENT_KERNEL_GENERAL, dst, 0, len, %%after)
    %jump(memset)
%%after:
%endmacro
