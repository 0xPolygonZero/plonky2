%macro blake_compression_internal_state_addr
    PUSH 0
%endmacro

%macro blake_compression_message_addr
    PUSH 16
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
        // stack: i, loc, ...
        DUP2
        DUP2
        // stack: i, loc, i, loc,...
        %blake_iv
        // stack: IV_i, loc, i, loc,...
        SWAP1
        // stack: loc, IV_i, i, loc,...
        %mstore_kernel_general
        // stack: i, loc,...
        %increment
        SWAP1
        %increment
        SWAP1
        // stack: i + 1, loc + 1,...
    %endrep
    %rep 4
        // stack: i, loc, val, next_val, next_val,...
        %stack (i, loc, val) -> (i, val, loc, i, loc)
        // stack: i, val, loc, i, loc, next_val,...
        %blake_iv
        // stack: IV_i, val, loc, i, loc, next_val,...
        XOR
        // stack: val ^ IV_i, loc, i, loc, next_val,...
        SWAP1
        // stack: loc, val ^ IV_i, i, loc, next_val,...
        %mstore_kernel_general
        // stack: i, loc, next_val,...
        %increment
        SWAP1
        %increment
        SWAP1
        // stack: i + 1, loc + 1, next_val,...
    %endrep
    // stack: 8, loc + 16, m_0, ..., m_15
    POP
    POP
    // stack: m_0, ..., m_15
    %blake_compression_message_addr
    // stack: addr, m_0, ..., m_15
    %rep 16
        
    %endrep
    PUSH 0
    // stack: round=0, m_0, ..., m_15
compression_loop:
    // stack: round, m_0, ..., m_15
    PUSH 0
    DUP2
    // stack: round, 0, round, m_0, ..., m_15
    %blake_permutation
    // stack: s[0], round, m_0, ..., m_15
    


