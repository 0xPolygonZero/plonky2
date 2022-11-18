global glv:
    PUSH 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001 dup1 dup1
    // we need to calculate k*b1. We require a full-width 512 bit multiplication,
    // see https://medium.com/wicketh/mathemagic-full-multiply-27650fec525d
    PUSH 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff // -1 n n k
    PUSH 0x24ccef014a773d2cf7a7bd9d4391eb18d dup6 mulmod // mm n n n k
    PUSH 0x24ccef014a773d2cf7a7bd9d4391eb18d dup6 mul    // bottom mm n n n k
    dup1 dup3 lt // x bottom mm n n n k
    swap2 sub sub // c2 n n n
    PUSH 0x89d3256894d213e3 mulmod // q2 n n k
    STOP
