%macro blake_initial_state
    %blake_iv(7)
    %blake_iv(6)
    %blake_iv(5)
    %blake_iv(4)
    %blake_iv(3)
    %blake_iv(2)
    %blake_iv(1)
    // stack: IV_1, IV_2, IV_3, IV_4, IV_5, IV_6, IV_7
    PUSH 0x01010040 // params: key = 00, digest_size = 64 = 0x40
    %blake_iv(0)
    XOR
    // stack: IV_0 ^ params, IV_1, IV_2, IV_3, IV_4, IV_5, IV_6, IV_7
    %stack () -> (0, 0)
    // stack: c_0 = 0, c_1 = 0, h_0, h_1, h_2, h_3, h_4, h_5, h_6, h_7
%endmacro
