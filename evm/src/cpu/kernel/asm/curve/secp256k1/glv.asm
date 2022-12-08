global glv:
    // One way of extracting short scalars k1, k2 is through lattice reduction
    // Calculate short basis scalars b1, b2 via the extended Euclidean algorithm
    // where b1 = a1.n + x and b2 = a2.n + y
    // and floor(b2/n) - floor(b1/n) = (a2 - a1) = n
    // One can then calculate floor(k.b2/n) = c1 and floor(-k.b1/n) = c2
    // c1, c2 calculated through Babai rounding and are half-length scalars
    // q1 = c1.b1 = k.b1.a1 and q2 = c2.b2 = -k.b2.a2
    // -(q1+q2) = -(k.b1.a1-k.b2.a2) = k2
    // k1 - k2\lambda = k
    // push group modulus n onto stack
    PUSH 0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141 dup1 dup1
    // we need to calculate k*b1. We require a full-width 512 bit multiplication,
    // see https://medium.com/wicketh/mathemagic-full-multiply-27650fec525d
    PUSH 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff // -1 n n n k
    PUSH 0xe4437ed6010e88286f547fa90abfe4c4 dup6 // k g1 -1 n n n k
    mulmod // mm=(k*g1 mod -1) n n n k
    PUSH 0xe4437ed6010e88286f547fa90abfe4c4 dup6 // k g1 mm=(k*g1 mod -1) n n n k
    mul    // bottom mm n n n k
    dup1 dup3  // m bottom bottom mm n n n k
    lt // (m < bottom) bottom mm n n n k
    swap2 //  mm bottom (m < bottom) n n n k
    sub sub // top = (mm - bottom - (m < bottom)) n n n k
    PUSH 0x3086d221a7d46bcde86c90e49284eb15 mulmod // q2 n n k

    PUSH 0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141
    PUSH 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff // -1 n q2 n n k
    PUSH 0x3086d221a7d46bcde86c90e49284eb15 dup7 mulmod // mm n q2 n n k
    PUSH 0x3086d221a7d46bcde86c90e49284eb15 dup7 mul    // bottom mm n q2 n n k
    dup1 dup3 lt // x bottom mm
    swap2 sub sub // c1 n q2 n n k
    PUSH 0xfffffffffffffffffffffffffffffffdd66b5e10ae3a1813507ddee3c5765c7e mulmod // q1 q2 n n k
    add %sub_check_underflow      // k2 underflow n k
    swap3                   // k underflow n k2
    PUSH 0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141 // n k underflow n k2
    dup5 PUSH 0x5363ad4cc05c30e0a5261c028812645a122e22ea20816678df02967c1b23bd72 // s k2 n k underflow n k2
    mulmod // (s*k2)%n k underflow n k2
    SWAP2 DUP1 %jumpi(underflowed)
    %stack (underflow, k, x, n, k2) -> (k, x, n, k2, underflow)
    addmod
    %stack (k1, k2, underflow, retdest) -> (retdest, underflow, k1, k2)
    JUMP

underflowed:
    // underflow k (s*k2)%n n k2
    %stack (u, k, x, n, k2) -> (n, x, k, n, k2, u)
    // TODO: Use SUBMOD here
    SUB ADDMOD
    %stack (k1, k2, underflow, retdest) -> (retdest, underflow, k1, k2)
    JUMP

%macro sub_check_underflow
    // stack: x, y
    DUP2 DUP2 LT
    // stack: x<y, x, y
    DUP1 ISZERO DUP2 DUP4 DUP6 SUB MUL
    // stack: (y-x)*(x<y), x>=y, x<y, x, y
    %stack (a, b, c, x, y) -> (x, y, b, a, c)
    SUB MUL ADD
    // stack: x-y if x>=y else y-x, x<y
%endmacro