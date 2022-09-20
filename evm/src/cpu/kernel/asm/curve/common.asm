global ret_zero_ec_mul:
    // stack: x, y, s, retdest
    %pop3
    // stack: retdest
    PUSH 0
    // stack: 0, retdest
    PUSH 0
    // stack: 0, 0, retdest
    SWAP2
    // stack: retdest, 0, 0
    JUMP
