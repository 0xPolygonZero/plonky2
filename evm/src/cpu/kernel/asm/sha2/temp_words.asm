%macro sha2_temp_word1
    // stack: e, f, g, h, K[i], W[i]
    dup1
    // stack: e, e, f, g, h, K[i], W[i]
    %sha2_bigsigma_1
    // stack: Sigma_1(e), e, f, g, h, K[i], W[i]
    swap3
    // stack: g, e, f, Sigma_1(e), h, K[i], W[i]
    swap2
    // stack: f, e, g, Sigma_1(e), h, K[i], W[i]
    swap1
    // stack: e, f, g, Sigma_1(e), h, K[i], W[i]
    %sha2_choice
    // stack: Ch(e, f, g), Sigma_1(e), h, K[i], W[i]
    add
    add
    add
    add
    // stack: Ch(e, f, g) + Sigma_1(e) + h + K[i] + W[i]
%endmacro

%macro sha2_temp_word2
    // stack: a, b, c
    dup1
    // stack: a, a, b, c
    %sha2_bigsigma_0
    // stack: Sigma_0(a), a, b, c
    swap3
    // stack: c, a, b, Sigma_0(a)
    %sha2_majority
    // stack: Maj(c, a, b), Sigma_0(a)
    add
    // stack: Maj(c, a, b) + Sigma_0(a)
%endmacro
