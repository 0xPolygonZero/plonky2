/// Note that Fj, Kj last for 16 iterations, but sj, rj update each call
///
/// def R(a, b, c, d, e, Fj, Kj, sj, rj, X):
///     a = u32(ROL(sj, u32(Fj(b, c, d) + a + X[rj] + Kj)) + e)
///     c = ROL(10, c)
///     return e, a, b, c, d

global R:
    jumpdest
    // stack: a, b, c, d, e, Fj, Kj, retdest
    dup4
    // stack: d, a, b, c, d, e, Fj, Kj, retdest
    dup4
    // stack: c, d, a, b, c, d, e, Fj, Kj, retdest
    dup4 
    // stack: b, c, d, a, b, c, d, e, Fj, Kj, retdest
    dup9
    // stack: Fj, b, c, d, a, b, c, d, e, Fj, Kj, retdest
    jump---------------------------------------------------------------------------TODO
    // stack: Fj(b, c, d), a, b, c, d, e, Fj, Kj, retdest
    add
    // stack: Fj(b, c, d) + a, b, c, d, e, Fj, Kj, retdest
    push X[rj]---------------------------------------------------------------------TODO
    // stack: X[rj], Fj(b, c, d) + a, b, c, d, e, Fj, Kj, retdest
    add
    // stack: X[rj] + Fj(b, c, d) + a, b, c, d, e, Fj, Kj, retdest
    dup7
    // stack: Kj, X[rj] + Fj(b, c, d) + a, b, c, d, e, Fj, Kj, retdest
    add
    // stack: Kj + X[rj] + Fj(b, c, d) + a, b, c, d, e, Fj, Kj, retdest
    %u32
    // stack: Kj + X[rj] + Fj(b, c, d) + a, b, c, d, e, Fj, Kj, retdest
    push sj------------------------------------------------------------------------TODO
    // stack: sj, Kj + X[rj] + Fj(b, c, d) + a, b, c, d, e, Fj, Kj, retdest
    push ROL
    // stack: ROL, sj, Kj + X[rj] + Fj(b, c, d) + a, b, c, d, e, Fj, Kj, retdest
    jump---------------------------------------------------------------------------TODO
    // stack: ROL(sj, Kj + X[rj] + Fj(b, c, d) + a), b, c, d, e, Fj, Kj, retdest
    dup5
    // stack: e, ROL(sj, Kj + X[rj] + Fj(b, c, d) + a), b, c, d, e, Fj, Kj, retdest
    add
    // stack: e + ROL(sj, Kj + X[rj] + Fj(b, c, d) + a), b, c, d, e, Fj, Kj, retdest
    %u32
    // stack: e + ROL(sj, Kj + X[rj] + Fj(b, c, d) + a), b, c, d, e, Fj, Kj, retdest
    swap2
    // stack: c, b, e + ROL(sj, Kj + X[rj] + Fj(b, c, d) + a), d, e, Fj, Kj, retdest
    push 10
    // stack: 10, c, b, e + ROL(sj, Kj + X[rj] + Fj(b, c, d) + a), d, e, Fj, Kj, retdest
    push ROL
    // stack: ROL, 10, c, b, e + ROL(sj, Kj + X[rj] + Fj(b, c, d) + a), d, e, Fj, Kj, retdest
    jump---------------------------------------------------------------------------TODO
    // stack: ROL(10, c), b, e + ROL(sj, Kj + X[rj] + Fj(b, c, d) + a), d, e, Fj, Kj, retdest
    %stack (c, b, a, d, e) -> (e, a, b, c, d)
    // stack: e, e + ROL(sj, Kj + X[rj] + Fj(b, c, d) + a), b, ROL(10, c), d, e, Fj, Kj, retdest


/// def ROL(n, x):
///     return (u32(x << n)) | (x >> (32 - n))

global Rol:
    jumpdest
    // stack: n, x, retdest
    swap1
    // stack: x, n, retdest
    dup1
    // stack: x, x, n, retdest
    dup3
    // stack: n, x, x, n, retdest
    push 32
    // stack: 32, n, x, x, n, retdest
    sub
    // stack: 32-n, x, x, n, retdest
    shr
    // stack: x >> (32-n), x, n, retdest
    swap2
    // stack: n, x, x >> (32-n), retdest
    shl
    // stack: x << n, x >> (32-n), retdest
    push 0xffffffff
    // stack: 0xffffffff, (x << n), x >> (32-n), retdest
    and
    // stack: (x << n) & 0xffffffff, x >> (32-n), retdest
    or
    // stack: ((x << n) & 0xffffffff) | (x >> (32-n)), retdest
    swap1
    // stack: retdest, ((x << n) & 0xffffffff) | (x >> (32-n))
    jump


/// def F0(x, y, z):
///     return x ^ y ^ z

global F0:
    jumpdest
    // stack: x, y, z, retdest
    xor
    // stack: x ^ y, z, retdest
    xor
    // stack: x ^ y ^ z, retdest
    swap1
    // stack: retdest, x ^ y ^ z
    jump


/// def F1(x, y, z):
///     return (x & y) | (u32(~x) & z)

global F1:
    jumpdest
    // stack: x, y, z, retdest
    dup1
    // stack: x, x, y, z, retdest
    swap2
    // stack: y, x, x, z, retdest
    and
    // stack: y & x, x, z, retdest
    swap2
    // stack: z, x, y & x, retdest
    swap1
    // stack: x, z, y & x, retdest
    %not_32
    // stack: ~x, z, y & x, retdest
    and
    // stack: ~x & z, y & x, retdest
    or
    // stack: (~x & z) | (y & x), retdest
    swap1
    // stack: retdest, (~x & z) | (y & x)
    jump


/// def F2(x, y, z):
///     return (x | u32(~y)) ^ z

global F2:
    jumpdest
    // stack: x, y, z, retdest
    swap1
    // stack: y, x, z, retdest
    %not_32
    // stack: ~y, x, z, retdest
    or
    // stack: ~y | x, z, retdest
    xor
    // stack: (~y | x) ^ z, retdest
    swap1
    // stack: retdest, (~y | x) ^ z
    jump


/// def F3(x, y, z):
///     return (x & z) | (u32(~z) & y)

global F3:
    jumpdest
    // stack: x, y, z, retdest
    dup3
    // stack: z, x, y, z, retdest
    and
    // stack: z & x, y, z, retdest
    swap2
    // stack: z, y, z & x, retdest
    %not_32
    // stack: ~z, y, z & x, retdest
    and
    // stack: ~z & y, z & x, retdest
    or
    // stack: (~z & y) | (z & x), retdest
    swap1
    // stack: retdest, (~z & y) | (z & x)
    jump 


/// def F4(x, y, z):
///     return x ^ (y | u32(~z))

global F4:
    jumpdest 
    // stack: x, y, z, retdest
    swap2
    // stack: z, y, x, retdest
    %not_32
    // stack: ~z, y, x, retdest
    or
    // stack: ~z | y, x, retdest
    xor
    // stack: (~z | y) ^ x, retdest
    swap1
    // stack: retdest, (~z | y) ^ x
    jump
