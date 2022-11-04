%macro blake_compression_internal_state_addr
    PUSH 0
%endmacro

global blake_compression:
    // stack: h_0, ..., h_7, t_0, t_1, f_0, f_1, m_0, ..., m_15
    %blake_compression_internal_state_addr
    // stack: start, h_0, ..., h_7, t_0, t_1, f_0, f_1, m_0, ..., m_15
    %rep 8
        SWAP1
        DUP2
        %mstore_kernel_general
        %increment
    %endrep
    // stack: start + 8, t_0, t_1, f_0, f_1, m_0, ..., m_15
    PUSH 0
    // stack: 0, start + 8, t_0, t_1, f_0, f_1, m_0, ..., m_15
    %rep 4
        
    %endrep

