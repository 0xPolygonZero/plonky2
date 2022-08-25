/// def rol(n, x):
///     return (u32(x << n)) | (x >> (32 - n))

global rol:
    jumpdest
    // stack:                        n, x, retdest
    swap1  dup1  dup3
    // stack:                  n, x, x, n, retdest
    push 32  sub
    // stack:               32-n, x, x, n, retdest
    shr
    // stack:           x >> (32-n), x, n, retdest
    swap2
    // stack:           n, x, x >> (32-n), retdest
    shl
    // stack:         x << n, x >> (32-n), retdest
    %u32
    // stack:    u32(x << n), x >> (32-n), retdest
    or
    // stack: u32(x << n) | (x >> (32-n)), retdest
    swap1  jump


%macro load_F:
  push 0
  %this_F(0,F0)
  %this_F(1,F1)
  %this_F(2,F2)
  %this_F(3,F3)
  %this_F(4,F4)
  %this_F(5,F4)
  %this_F(6,F3)
  %this_F(7,F2)
  %this_F(8,F1)
  %this_F(9,F0)
%endmacro


%macro this_F(i, F)
  // stack: acc, rnd
  dup2
  // stack: rnd, acc, rnd
  %eq_const(i)
  // stack: rnd==i, acc, j
  %mul_const(result)
  // stack: (rnd==i)*F, acc, rnd
  add
  acc + (rnd==j)*result, rnd
%endmacro


/// def F0(x, y, z):
///     return x ^ y ^ z

global F0:
    jumpdest
    // stack: x , y , z, retdest
    xor
    // stack: x ^ y , z, retdest
    xor
    // stack: x ^ y ^ z, retdest
    swap1  jump


/// def F1(x, y, z):
///     return (x & y) | (u32(~x) & z)

global F1:
    jumpdest
    // stack:            x, y, z, retdest
    dup1
    // stack:         x, x, y, z, retdest
    swap2
    // stack:         y, x, x, z, retdest
    and
    // stack:        y & x, x, z, retdest
    swap2
    // stack:        z, x, y & x, retdest
    swap1
    // stack:        x, z, y & x, retdest
    %not_32
    // stack:       ~x, z, y & x, retdest
    and
    // stack:      ~x & z, y & x, retdest
    or
    // stack: (~x & z) | (y & x), retdest
    swap1  jump


/// def F2(x, y, z):
///     return (x | u32(~y)) ^ z

global F2:
    jumpdest
    // stack:      x, y, z, retdest
    swap1
    // stack:      y, x, z, retdest
    %not_32
    // stack:     ~y, x, z, retdest
    or
    // stack:    ~y | x, z, retdest
    xor
    // stack: (~y | x) ^ z, retdest
    swap1  jump


/// def F3(x, y, z):
///     return (x & z) | (u32(~z) & y)

global F3:
    jumpdest
    // stack:            x, y, z, retdest
    dup3
    // stack:         z, x, y, z, retdest
    and
    // stack:        z & x, y, z, retdest
    swap2
    // stack:        z, y, z & x, retdest
    %not_32
    // stack:       ~z, y, z & x, retdest
    and
    // stack:      ~z & y, z & x, retdest
    or
    // stack: (~z & y) | (z & x), retdest
    swap1  jump 


/// def F4(x, y, z):
///     return x ^ (y | u32(~z))

global F4:
    jumpdest 
    // stack:      x, y, z, retdest
    swap2
    // stack:      z, y, x, retdest
    %not_32
    // stack:     ~z, y, x, retdest
    or
    // stack:    ~z | y, x, retdest
    xor
    // stack: (~z | y) ^ x, retdest
    swap1  jump
