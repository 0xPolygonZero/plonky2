%macro pop_bit
    // stack: key
    DUP1 %shr_const(1)
    // stack: key>>1, key
    SWAP1 %and_const(1)
    // stack: key&1, key>>1
%endmacro
