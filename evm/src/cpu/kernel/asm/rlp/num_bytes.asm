// Get the number of bytes required to represent the given scalar.
// Note that we define num_bytes(0) to be 1.

global num_bytes:
    // stack: x, retdest
    DUP1 PUSH  0 BYTE %jumpi(return_32)
    DUP1 PUSH  1 BYTE %jumpi(return_31)
    DUP1 PUSH  2 BYTE %jumpi(return_30)
    DUP1 PUSH  3 BYTE %jumpi(return_29)
    DUP1 PUSH  4 BYTE %jumpi(return_28)
    DUP1 PUSH  5 BYTE %jumpi(return_27)
    DUP1 PUSH  6 BYTE %jumpi(return_26)
    DUP1 PUSH  7 BYTE %jumpi(return_25)
    DUP1 PUSH  8 BYTE %jumpi(return_24)
    DUP1 PUSH  9 BYTE %jumpi(return_23)
    DUP1 PUSH 10 BYTE %jumpi(return_22)
    DUP1 PUSH 11 BYTE %jumpi(return_21)
    DUP1 PUSH 12 BYTE %jumpi(return_20)
    DUP1 PUSH 13 BYTE %jumpi(return_19)
    DUP1 PUSH 14 BYTE %jumpi(return_18)
    DUP1 PUSH 15 BYTE %jumpi(return_17)
    DUP1 PUSH 16 BYTE %jumpi(return_16)
    DUP1 PUSH 17 BYTE %jumpi(return_15)
    DUP1 PUSH 18 BYTE %jumpi(return_14)
    DUP1 PUSH 19 BYTE %jumpi(return_13)
    DUP1 PUSH 20 BYTE %jumpi(return_12)
    DUP1 PUSH 21 BYTE %jumpi(return_11)
    DUP1 PUSH 22 BYTE %jumpi(return_10)
    DUP1 PUSH 23 BYTE %jumpi(return_9)
    DUP1 PUSH 24 BYTE %jumpi(return_8)
    DUP1 PUSH 25 BYTE %jumpi(return_7)
    DUP1 PUSH 26 BYTE %jumpi(return_6)
    DUP1 PUSH 27 BYTE %jumpi(return_5)
    DUP1 PUSH 28 BYTE %jumpi(return_4)
    DUP1 PUSH 29 BYTE %jumpi(return_3)
         PUSH 30 BYTE %jumpi(return_2)

    // If we got all the way here, each byte was zero, except possibly the least
    // significant byte, which we didn't check. Either way, the result is 1.
    // stack: retdest
    PUSH 1
    SWAP1
    JUMP

return_2:      PUSH  2 SWAP1 JUMP
return_3:  POP PUSH  3 SWAP1 JUMP
return_4:  POP PUSH  4 SWAP1 JUMP
return_5:  POP PUSH  5 SWAP1 JUMP
return_6:  POP PUSH  6 SWAP1 JUMP
return_7:  POP PUSH  7 SWAP1 JUMP
return_8:  POP PUSH  8 SWAP1 JUMP
return_9:  POP PUSH  9 SWAP1 JUMP
return_10: POP PUSH 10 SWAP1 JUMP
return_11: POP PUSH 11 SWAP1 JUMP
return_12: POP PUSH 12 SWAP1 JUMP
return_13: POP PUSH 13 SWAP1 JUMP
return_14: POP PUSH 14 SWAP1 JUMP
return_15: POP PUSH 15 SWAP1 JUMP
return_16: POP PUSH 16 SWAP1 JUMP
return_17: POP PUSH 17 SWAP1 JUMP
return_18: POP PUSH 18 SWAP1 JUMP
return_19: POP PUSH 19 SWAP1 JUMP
return_20: POP PUSH 20 SWAP1 JUMP
return_21: POP PUSH 21 SWAP1 JUMP
return_22: POP PUSH 22 SWAP1 JUMP
return_23: POP PUSH 23 SWAP1 JUMP
return_24: POP PUSH 24 SWAP1 JUMP
return_25: POP PUSH 25 SWAP1 JUMP
return_26: POP PUSH 26 SWAP1 JUMP
return_27: POP PUSH 27 SWAP1 JUMP
return_28: POP PUSH 28 SWAP1 JUMP
return_29: POP PUSH 29 SWAP1 JUMP
return_30: POP PUSH 30 SWAP1 JUMP
return_31: POP PUSH 31 SWAP1 JUMP
return_32: POP PUSH 32 SWAP1 JUMP

// Convenience macro to call num_bytes and return where we left off.
%macro num_bytes
    %stack (x) -> (x, %%after)
    %jump(num_bytes)
%%after:
%endmacro
