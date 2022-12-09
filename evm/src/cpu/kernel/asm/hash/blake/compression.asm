%macro blake_compression_internal_state_addr
    PUSH 0
%endmacro

global blake_compression:
    // stack: h_0, ..., h_7, t_0, t_1, f_0, f_1, m_0, ..., m_15
    