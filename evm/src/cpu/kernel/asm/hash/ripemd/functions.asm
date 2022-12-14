/// def rol(n, x):
///     return (u32(x << n)) | (x >> (32 - n))

global rol:
    // stack:                        n, x, retdest
    SWAP1  
    DUP1  
    DUP3
    // stack:                  n, x, x, n, retdest
    PUSH 32  
    SUB
    // stack:               32-n, x, x, n, retdest
    SHR
    // stack:           x >> (32-n), x, n, retdest
    SWAP2
    // stack:           n, x, x >> (32-n), retdest
    SHL
    // stack:         x << n, x >> (32-n), retdest
    %as_u32
    // stack:    u32(x << n), x >> (32-n), retdest
    OR
    // stack: u32(x << n) | (x >> (32-n)), retdest
    SWAP1  
    JUMP

// def push_f(rnd):
//     Fs = [F0, F1, F2, F3, F4, F4, F3, F2, F1, F0]
//     acc = 0
//     for i, F in enumerate(Fs):
//         acc += (i==rnd)*F
//     return acc, rnd
//
// %this_f(i,F) enacts
//     acc += (i==rnd)*F

%macro push_f
    // stack:    rnd
    PUSH 0
    %this_f(0,F0)
    %this_f(1,F1)
    %this_f(2,F2)
    %this_f(3,F3)
    %this_f(4,F4)
    %this_f(5,F4)
    %this_f(6,F3)
    %this_f(7,F2)
    %this_f(8,F1)
    %this_f(9,F0)
    // stack: F, rnd
%endmacro

%macro this_f(i, F)
  // stack:              acc, rnd
  DUP2
  // stack:  rnd       , acc, rnd
  %eq_const($i)
  // stack:  rnd==i    , acc, rnd
  %mul_const($F)
  // stack: (rnd==i)*F , acc, rnd
  ADD
  // stack: (rnd==j)*F + acc, rnd
%endmacro

/// def F0(x, y, z):
///     return x ^ y ^ z

global F0: 
    // stack: x , y , z, retdest
    XOR
    // stack: x ^ y , z, retdest
    XOR
    // stack: x ^ y ^ z, retdest
    SWAP1  
    JUMP

/// def F1(x, y, z):
///     return (x & y) | (u32(~x) & z)

global F1:  
    // stack:            x, y, z, retdest
    DUP1
    // stack:        x,  x, y, z, retdest
    SWAP2
    // stack:        y,  x, x, z, retdest
    AND
    // stack:        y & x, x, z, retdest
    SWAP2
    // stack:   z,  x,    y & x , retdest
    SWAP1
    // stack:   x,  z,    y & x , retdest
    %not_u32
    // stack:  ~x,  z,    y & x , retdest
    AND
    // stack:  ~x & z  ,  y & x , retdest
    OR
    // stack: (~x & z) | (y & x), retdest
    SWAP1  
    JUMP

/// def F2(x, y, z):
///     return (x | u32(~y)) ^ z

global F2:
    // stack:   x , y,   z, retdest
    SWAP1
    // stack:   y , x,   z, retdest
    %not_u32
    // stack:  ~y , x ,  z, retdest
    OR
    // stack:  ~y | x ,  z, retdest
    XOR
    // stack: (~y | x) ^ z, retdest
    SWAP1  
    JUMP

/// def F3(x, y, z):
///     return (x & z) | (u32(~z) & y)

global F3: 
    // stack:       x,    y , z , retdest
    DUP3
    // stack:   z , x,    y , z , retdest
    AND
    // stack:   z & x,    y , z , retdest
    SWAP2
    // stack:   z,  y,    z & x , retdest
    %not_u32
    // stack:  ~z , y,    z & x , retdest
    AND
    // stack:  ~z & y,    z & x , retdest
    OR
    // stack: (~z & y) | (z & x), retdest
    SWAP1  
    JUMP 

/// def F4(x, y, z):
///     return x ^ (y | u32(~z))

global F4:
    // stack:   x,  y,   z, retdest
    SWAP2
    // stack:   z,  y,   x, retdest
    %not_u32
    // stack:  ~z,  y,   x, retdest
    OR
    // stack:  ~z | y,   x, retdest
    XOR
    // stack: (~z | y) ^ x, retdest
    SWAP1
    JUMP
